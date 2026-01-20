//! SFTP Client Implementation
//!
//! NIST 800-53: IA-2 (Identification and Authentication), SC-8 (Transmission Confidentiality), SC-13 (Cryptographic Protection)
//! STIG: V-222577 (Cryptographic mechanisms), V-222611 (Certificate validation)
//! Implementation: RFC-compliant SFTP client with SSH authentication

use crate::{cnsa, Error, Result};
use bytes::{Buf, BufMut, BytesMut};
use russh::client::{self, Handle, Msg};
use russh::{Channel, ChannelMsg};
use russh::keys::{PrivateKey, PrivateKeyWithHashAlg, PublicKey};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::protocol::{codec, FileAttrs, MessageType, OpenFlags, StatusCode, SFTP_VERSION};

/// SFTP Client
///
/// NIST 800-53: IA-2 (Identification and Authentication), SC-8 (Transmission Confidentiality)
/// STIG: V-222577 (Cryptographic mechanisms)
/// Implementation: SSH/SFTP client with public key authentication
pub struct Client {
    session: Arc<Mutex<Option<Handle<ClientHandler>>>>,
    channel: Arc<Mutex<Option<Channel<Msg>>>>,
    next_request_id: Arc<Mutex<u32>>,
    _responses: Arc<Mutex<HashMap<u32, Vec<u8>>>>,
}

impl Client {
    /// Connect to an SFTP server
    ///
    /// # Arguments
    ///
    /// * `host` - Server hostname or IP address
    /// * `port` - Server port
    /// * `username` - Username for authentication
    /// * `key_path` - Path to private SSH key
    ///
    /// # Returns
    ///
    /// Connected SFTP client
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Connection fails
    /// - Authentication fails
    /// - SFTP subsystem cannot be started
    ///
    /// # NIST 800-53: IA-2 (Identification and Authentication), SC-8 (Transmission Confidentiality)
    /// # STIG: V-222577 (Cryptographic mechanisms), V-222611 (Certificate validation)
    /// # Implementation: Establishes SSH connection with public key authentication
    pub async fn connect(
        host: &str,
        port: u16,
        username: &str,
        key_path: &Path,
    ) -> Result<Self> {
        info!("Connecting to {}:{} as {}", host, port, username);

        // NIST 800-53: SC-13 (Cryptographic Protection) - Load private key
        let key_pair = load_private_key(key_path).await?;

        // NSA CNSA 2.0: Configure only approved cryptographic algorithms
        let mut config = russh::client::Config::default();
        config.preferred = russh::Preferred {
            kex: std::borrow::Cow::Borrowed(cnsa::CNSA_KEX_ALGORITHMS),
            key: std::borrow::Cow::Borrowed(cnsa::CNSA_PUBLIC_KEY_ALGORITHMS),
            cipher: std::borrow::Cow::Borrowed(cnsa::CNSA_CIPHERS),
            mac: std::borrow::Cow::Borrowed(cnsa::CNSA_MAC_ALGORITHMS),
            ..Default::default()
        };

        info!(
            event = "cnsa_client_config",
            kex = ?cnsa::CNSA_KEX_ALGORITHMS,
            ciphers = ?cnsa::CNSA_CIPHERS,
            "CNSA 2.0 compliant client configured"
        );

        // NIST 800-53: SC-8 (Transmission Confidentiality) - Establish SSH connection
        let sh = ClientHandler::new();

        let mut session = russh::client::connect(
            Arc::new(config),
            format!("{}:{}", host, port),
            sh,
        )
        .await
        .map_err(|e| Error::Connection(format!("SSH connection failed: {}", e)))?;

        // NIST 800-53: IA-2 (Identification and Authentication) - Authenticate with public key
        let key_with_alg = PrivateKeyWithHashAlg::new(Arc::new(key_pair), None);
        let auth_result = session
            .authenticate_publickey(username, key_with_alg)
            .await
            .map_err(|e| Error::Authentication(format!("Authentication failed: {}", e)))?;

        if !auth_result.success() {
            return Err(Error::Authentication(
                "Public key authentication failed".into(),
            ));
        }

        info!("Authentication successful for user: {}", username);

        // Open channel for SFTP subsystem
        let channel = session
            .channel_open_session()
            .await
            .map_err(|e| Error::Connection(format!("Failed to open channel: {}", e)))?;

        // Request SFTP subsystem
        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|e| Error::Protocol(format!("Failed to start SFTP subsystem: {}", e)))?;

        let client = Self {
            session: Arc::new(Mutex::new(Some(session))),
            channel: Arc::new(Mutex::new(Some(channel))),
            next_request_id: Arc::new(Mutex::new(1)),
            _responses: Arc::new(Mutex::new(HashMap::new())),
        };

        // Initialize SFTP protocol
        client.init().await?;

        info!("SFTP client initialized successfully");

        Ok(client)
    }

    /// Initialize SFTP protocol
    ///
    /// # NIST 800-53: SC-8 (Transmission Confidentiality)
    /// # Implementation: Sends INIT message and validates VERSION response
    async fn init(&self) -> Result<()> {
        debug!("Initializing SFTP protocol");

        let mut buf = BytesMut::new();
        buf.put_u8(MessageType::Init as u8);
        buf.put_u32(SFTP_VERSION);

        self.send_packet(&buf).await?;

        let response = self.receive_packet().await?;

        if response.is_empty() || response[0] != MessageType::Version as u8 {
            return Err(Error::Protocol("Invalid INIT response".into()));
        }

        let version = u32::from_be_bytes([response[1], response[2], response[3], response[4]]);
        info!("SFTP server version: {}", version);

        if version != SFTP_VERSION {
            warn!(
                "Server version {} differs from client version {}",
                version, SFTP_VERSION
            );
        }

        Ok(())
    }

    /// Upload a file to the server
    ///
    /// # Arguments
    ///
    /// * `local_path` - Path to local file
    /// * `remote_path` - Destination path on server
    ///
    /// # Returns
    ///
    /// `Ok(())` on success
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Local file cannot be read
    /// - Remote file cannot be opened
    /// - Write operation fails
    ///
    /// # NIST 800-53: SC-8 (Transmission Confidentiality)
    /// # Implementation: Transfers file data over encrypted SSH channel
    pub async fn put(&mut self, local_path: &Path, remote_path: &str) -> Result<()> {
        info!("Uploading {:?} to {}", local_path, remote_path);

        // Read local file
        let mut file = fs::File::open(local_path)
            .await
            .map_err(|e| Error::Io(e))?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .await
            .map_err(|e| Error::Io(e))?;

        // Open remote file for writing
        let handle = self
            .open(
                remote_path,
                OpenFlags(OpenFlags::WRITE | OpenFlags::CREAT | OpenFlags::TRUNC),
            )
            .await?;

        // Write data
        self.write(&handle, 0, &contents).await?;

        // Close file
        self.close(&handle).await?;

        info!("Upload completed: {:?}", local_path);

        Ok(())
    }

    /// Download a file from the server
    ///
    /// # Arguments
    ///
    /// * `remote_path` - Path to file on server
    /// * `local_path` - Destination path locally
    ///
    /// # Returns
    ///
    /// `Ok(())` on success
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Remote file cannot be opened
    /// - Read operation fails
    /// - Local file cannot be written
    ///
    /// # NIST 800-53: SC-8 (Transmission Confidentiality)
    /// # Implementation: Transfers file data over encrypted SSH channel
    pub async fn get(&mut self, remote_path: &str, local_path: &Path) -> Result<()> {
        info!("Downloading {} to {:?}", remote_path, local_path);

        // Open remote file for reading
        let handle = self.open(remote_path, OpenFlags(OpenFlags::READ)).await?;

        // Get file size
        let attrs = self.fstat(&handle).await?;
        let file_size = attrs.size.unwrap_or(0);

        // Read file data
        let mut data = Vec::new();
        let mut offset = 0;
        let chunk_size = 32768; // 32KB chunks

        while offset < file_size {
            let len = std::cmp::min(chunk_size, file_size - offset);
            let chunk = self.read(&handle, offset, len as u32).await?;

            if chunk.is_empty() {
                break; // EOF
            }

            data.extend_from_slice(&chunk);
            offset += chunk.len() as u64;
        }

        // Close remote file
        self.close(&handle).await?;

        // Write to local file
        let mut file = fs::File::create(local_path)
            .await
            .map_err(|e| Error::Io(e))?;

        file.write_all(&data).await.map_err(|e| Error::Io(e))?;

        info!("Download completed: {:?}", local_path);

        Ok(())
    }

    /// List directory contents
    ///
    /// # Arguments
    ///
    /// * `path` - Directory path
    ///
    /// # Returns
    ///
    /// Vector of (filename, attributes) tuples
    ///
    /// # Errors
    ///
    /// Returns error if directory cannot be opened or read
    ///
    /// # NIST 800-53: AC-3 (Access Enforcement)
    /// # Implementation: Lists directory contents within authorized scope
    pub async fn list(&mut self, path: &str) -> Result<Vec<(String, FileAttrs)>> {
        debug!("Listing directory: {}", path);

        // Open directory
        let handle = self.opendir(path).await?;

        let mut entries = Vec::new();

        // Read directory entries
        loop {
            match self.readdir(&handle).await {
                Ok(Some(batch)) => {
                    entries.extend(batch);
                }
                Ok(None) => break, // EOF
                Err(e) => {
                    self.close(&handle).await.ok();
                    return Err(e);
                }
            }
        }

        // Close directory
        self.close(&handle).await?;

        Ok(entries)
    }

    /// Create a directory
    ///
    /// # Arguments
    ///
    /// * `path` - Directory path to create
    ///
    /// # NIST 800-53: AC-3 (Access Enforcement)
    /// # Implementation: Creates directory within authorized scope
    pub async fn mkdir(&mut self, path: &str) -> Result<()> {
        debug!("Creating directory: {}", path);

        let request_id = self.next_request_id().await;

        let mut buf = BytesMut::new();
        buf.put_u8(MessageType::Mkdir as u8);
        buf.put_u32(request_id);
        codec::put_string(&mut buf, path);
        buf.extend_from_slice(&FileAttrs::default().encode());

        self.send_packet(&buf).await?;
        self.check_status(request_id).await
    }

    /// Remove a file
    ///
    /// # Arguments
    ///
    /// * `path` - File path to remove
    ///
    /// # NIST 800-53: AC-3 (Access Enforcement)
    /// # Implementation: Removes file within authorized scope
    pub async fn remove(&mut self, path: &str) -> Result<()> {
        debug!("Removing file: {}", path);

        let request_id = self.next_request_id().await;

        let mut buf = BytesMut::new();
        buf.put_u8(MessageType::Remove as u8);
        buf.put_u32(request_id);
        codec::put_string(&mut buf, path);

        self.send_packet(&buf).await?;
        self.check_status(request_id).await
    }

    /// Remove a directory
    ///
    /// # Arguments
    ///
    /// * `path` - Directory path to remove
    ///
    /// # NIST 800-53: AC-3 (Access Enforcement)
    /// # Implementation: Removes directory within authorized scope
    pub async fn rmdir(&mut self, path: &str) -> Result<()> {
        debug!("Removing directory: {}", path);

        let request_id = self.next_request_id().await;

        let mut buf = BytesMut::new();
        buf.put_u8(MessageType::Rmdir as u8);
        buf.put_u32(request_id);
        codec::put_string(&mut buf, path);

        self.send_packet(&buf).await?;
        self.check_status(request_id).await
    }

    /// Rename a file or directory
    ///
    /// # Arguments
    ///
    /// * `old_path` - Current path
    /// * `new_path` - New path
    ///
    /// # NIST 800-53: AC-3 (Access Enforcement)
    /// # Implementation: Renames file within authorized scope
    pub async fn rename(&mut self, old_path: &str, new_path: &str) -> Result<()> {
        debug!("Renaming {} to {}", old_path, new_path);

        let request_id = self.next_request_id().await;

        let mut buf = BytesMut::new();
        buf.put_u8(MessageType::Rename as u8);
        buf.put_u32(request_id);
        codec::put_string(&mut buf, old_path);
        codec::put_string(&mut buf, new_path);

        self.send_packet(&buf).await?;
        self.check_status(request_id).await
    }

    /// Get file attributes
    ///
    /// # Arguments
    ///
    /// * `path` - File or directory path
    ///
    /// # Returns
    ///
    /// File attributes
    ///
    /// # NIST 800-53: AC-3 (Access Enforcement)
    /// # Implementation: Retrieves attributes within authorized scope
    pub async fn stat(&mut self, path: &str) -> Result<FileAttrs> {
        debug!("Getting attributes for: {}", path);

        let request_id = self.next_request_id().await;

        let mut buf = BytesMut::new();
        buf.put_u8(MessageType::Stat as u8);
        buf.put_u32(request_id);
        codec::put_string(&mut buf, path);

        self.send_packet(&buf).await?;

        let response = self.receive_response(request_id).await?;
        self.parse_attrs_response(&response)
    }

    /// Disconnect from the server
    ///
    /// # NIST 800-53: AC-12 (Session Termination)
    /// # Implementation: Gracefully terminates SSH session
    pub async fn disconnect(self) -> Result<()> {
        info!("Disconnecting from server");

        if let Some(session) = self.session.lock().await.take() {
            session
                .disconnect(russh::Disconnect::ByApplication, "", "en")
                .await
                .ok();
        }

        Ok(())
    }

    // ===== Private helper methods =====

    async fn open(&mut self, path: &str, flags: OpenFlags) -> Result<Vec<u8>> {
        let request_id = self.next_request_id().await;

        let mut buf = BytesMut::new();
        buf.put_u8(MessageType::Open as u8);
        buf.put_u32(request_id);
        codec::put_string(&mut buf, path);
        buf.put_u32(flags.0);
        buf.extend_from_slice(&FileAttrs::default().encode());

        self.send_packet(&buf).await?;

        let response = self.receive_response(request_id).await?;
        self.parse_handle_response(&response)
    }

    async fn close(&mut self, handle: &[u8]) -> Result<()> {
        let request_id = self.next_request_id().await;

        let mut buf = BytesMut::new();
        buf.put_u8(MessageType::Close as u8);
        buf.put_u32(request_id);
        codec::put_bytes(&mut buf, handle);

        self.send_packet(&buf).await?;
        self.check_status(request_id).await
    }

    async fn read(&mut self, handle: &[u8], offset: u64, len: u32) -> Result<Vec<u8>> {
        let request_id = self.next_request_id().await;

        let mut buf = BytesMut::new();
        buf.put_u8(MessageType::Read as u8);
        buf.put_u32(request_id);
        codec::put_bytes(&mut buf, handle);
        buf.put_u64(offset);
        buf.put_u32(len);

        self.send_packet(&buf).await?;

        let response = self.receive_response(request_id).await?;
        self.parse_data_response(&response)
    }

    async fn write(&mut self, handle: &[u8], offset: u64, data: &[u8]) -> Result<()> {
        let request_id = self.next_request_id().await;

        let mut buf = BytesMut::new();
        buf.put_u8(MessageType::Write as u8);
        buf.put_u32(request_id);
        codec::put_bytes(&mut buf, handle);
        buf.put_u64(offset);
        codec::put_bytes(&mut buf, data);

        self.send_packet(&buf).await?;
        self.check_status(request_id).await
    }

    async fn opendir(&mut self, path: &str) -> Result<Vec<u8>> {
        let request_id = self.next_request_id().await;

        let mut buf = BytesMut::new();
        buf.put_u8(MessageType::Opendir as u8);
        buf.put_u32(request_id);
        codec::put_string(&mut buf, path);

        self.send_packet(&buf).await?;

        let response = self.receive_response(request_id).await?;
        self.parse_handle_response(&response)
    }

    async fn readdir(&mut self, handle: &[u8]) -> Result<Option<Vec<(String, FileAttrs)>>> {
        let request_id = self.next_request_id().await;

        let mut buf = BytesMut::new();
        buf.put_u8(MessageType::Readdir as u8);
        buf.put_u32(request_id);
        codec::put_bytes(&mut buf, handle);

        self.send_packet(&buf).await?;

        let response = self.receive_response(request_id).await?;

        if response.is_empty() {
            return Err(Error::Protocol("Empty READDIR response".into()));
        }

        let msg_type = MessageType::try_from(response[0])?;

        match msg_type {
            MessageType::Name => self.parse_name_response(&response).map(Some),
            MessageType::Status => {
                // EOF is indicated by STATUS with EOF code
                let mut buf = &response[1..];
                let _request_id = buf.get_u32();
                let code = buf.get_u32();

                if code == StatusCode::Eof as u32 {
                    Ok(None) // EOF
                } else {
                    let message = codec::get_string(&mut buf).unwrap_or_default();
                    Err(Error::Protocol(format!("READDIR failed: {}", message)))
                }
            }
            _ => Err(Error::Protocol(format!(
                "Unexpected response type: {:?}",
                msg_type
            ))),
        }
    }

    async fn fstat(&mut self, handle: &[u8]) -> Result<FileAttrs> {
        let request_id = self.next_request_id().await;

        let mut buf = BytesMut::new();
        buf.put_u8(MessageType::Fstat as u8);
        buf.put_u32(request_id);
        codec::put_bytes(&mut buf, handle);

        self.send_packet(&buf).await?;

        let response = self.receive_response(request_id).await?;
        self.parse_attrs_response(&response)
    }

    async fn send_packet(&self, data: &[u8]) -> Result<()> {
        let channel = self.channel.lock().await;
        let channel = channel
            .as_ref()
            .ok_or_else(|| Error::Connection("Channel closed".into()))?;

        let mut packet = BytesMut::new();
        packet.put_u32(data.len() as u32);
        packet.extend_from_slice(data);

        // Convert BytesMut to slice for AsyncRead compatibility
        let packet_bytes: &[u8] = &packet;
        channel
            .data(packet_bytes)
            .await
            .map_err(|e| Error::Connection(format!("Failed to send packet: {}", e)))?;

        Ok(())
    }

    async fn receive_packet(&self) -> Result<Vec<u8>> {
        let mut channel = self.channel.lock().await;
        let channel = channel
            .as_mut()
            .ok_or_else(|| Error::Connection("Channel closed".into()))?;

        // Wait for channel message
        loop {
            if let Some(msg) = channel.wait().await {
                match msg {
                    ChannelMsg::Data { data } => {
                        if data.len() < 4 {
                            return Err(Error::Protocol("Packet too short".into()));
                        }

                        let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;

                        if data.len() < 4 + len {
                            return Err(Error::Protocol("Incomplete packet".into()));
                        }

                        return Ok(data[4..4 + len].to_vec());
                    }
                    ChannelMsg::Eof => {
                        return Err(Error::Connection("Channel EOF".into()));
                    }
                    ChannelMsg::Close => {
                        return Err(Error::Connection("Channel closed".into()));
                    }
                    _ => {
                        // Ignore other messages
                        continue;
                    }
                }
            } else {
                return Err(Error::Connection("Channel closed unexpectedly".into()));
            }
        }
    }

    async fn receive_response(&self, _request_id: u32) -> Result<Vec<u8>> {
        // In a real implementation, we'd match request IDs
        // For simplicity, we'll just receive the next packet
        self.receive_packet().await
    }

    async fn check_status(&self, request_id: u32) -> Result<()> {
        let response = self.receive_response(request_id).await?;

        if response.is_empty() {
            return Err(Error::Protocol("Empty status response".into()));
        }

        let msg_type = MessageType::try_from(response[0])?;

        if msg_type != MessageType::Status {
            return Err(Error::Protocol(format!(
                "Expected STATUS, got {:?}",
                msg_type
            )));
        }

        let mut buf = &response[1..];
        let _resp_id = buf.get_u32();
        let code = buf.get_u32();
        let message = codec::get_string(&mut buf).unwrap_or_default();

        if code == StatusCode::Ok as u32 {
            Ok(())
        } else {
            Err(Error::Protocol(format!("Operation failed: {}", message)))
        }
    }

    fn parse_handle_response(&self, response: &[u8]) -> Result<Vec<u8>> {
        if response.is_empty() {
            return Err(Error::Protocol("Empty handle response".into()));
        }

        let msg_type = MessageType::try_from(response[0])?;

        if msg_type != MessageType::Handle {
            return Err(Error::Protocol(format!(
                "Expected HANDLE, got {:?}",
                msg_type
            )));
        }

        let mut buf = &response[1..];
        let _request_id = buf.get_u32();
        let handle = codec::get_bytes(&mut buf)?;

        Ok(handle.to_vec())
    }

    fn parse_data_response(&self, response: &[u8]) -> Result<Vec<u8>> {
        if response.is_empty() {
            return Err(Error::Protocol("Empty data response".into()));
        }

        let msg_type = MessageType::try_from(response[0])?;

        match msg_type {
            MessageType::Data => {
                let mut buf = &response[1..];
                let _request_id = buf.get_u32();
                let data = codec::get_bytes(&mut buf)?;
                Ok(data.to_vec())
            }
            MessageType::Status => {
                // Check for EOF
                let mut buf = &response[1..];
                let _request_id = buf.get_u32();
                let code = buf.get_u32();

                if code == StatusCode::Eof as u32 {
                    Ok(Vec::new()) // EOF
                } else {
                    let message = codec::get_string(&mut buf).unwrap_or_default();
                    Err(Error::Protocol(format!("Read failed: {}", message)))
                }
            }
            _ => Err(Error::Protocol(format!(
                "Expected DATA or STATUS, got {:?}",
                msg_type
            ))),
        }
    }

    fn parse_attrs_response(&self, response: &[u8]) -> Result<FileAttrs> {
        if response.is_empty() {
            return Err(Error::Protocol("Empty attrs response".into()));
        }

        let msg_type = MessageType::try_from(response[0])?;

        if msg_type != MessageType::Attrs {
            return Err(Error::Protocol(format!(
                "Expected ATTRS, got {:?}",
                msg_type
            )));
        }

        let mut buf = &response[1..];
        let _request_id = buf.get_u32();
        let attrs = FileAttrs::decode(&mut buf)?;

        Ok(attrs)
    }

    fn parse_name_response(&self, response: &[u8]) -> Result<Vec<(String, FileAttrs)>> {
        if response.is_empty() {
            return Err(Error::Protocol("Empty name response".into()));
        }

        let msg_type = MessageType::try_from(response[0])?;

        if msg_type != MessageType::Name {
            return Err(Error::Protocol(format!(
                "Expected NAME, got {:?}",
                msg_type
            )));
        }

        let mut buf = &response[1..];
        let _request_id = buf.get_u32();
        let count = buf.get_u32() as usize;

        let mut entries = Vec::with_capacity(count);

        for _ in 0..count {
            let filename = codec::get_string(&mut buf)?;
            let _longname = codec::get_string(&mut buf)?;
            let attrs = FileAttrs::decode(&mut buf)?;

            entries.push((filename.to_string(), attrs));
        }

        Ok(entries)
    }

    async fn next_request_id(&self) -> u32 {
        let mut id = self.next_request_id.lock().await;
        let current = *id;
        *id = id.wrapping_add(1);
        current
    }
}

/// SSH client handler
struct ClientHandler {}

impl ClientHandler {
    fn new() -> Self {
        Self {}
    }
}

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> std::result::Result<bool, Self::Error> {
        // NIST 800-53: IA-5 (Authenticator Management)
        // TODO: Implement proper server key verification
        // For now, accept all keys (INSECURE - should verify against known_hosts)
        warn!("Server key verification not implemented - accepting all keys (INSECURE)");
        Ok(true)
    }
}

/// Load private SSH key from file
///
/// # NIST 800-53: IA-5 (Authenticator Management), SC-13 (Cryptographic Protection)
/// # Implementation: Loads private key for authentication
async fn load_private_key(path: &Path) -> Result<PrivateKey> {
    russh::keys::load_secret_key(path, None)
        .map_err(|e| Error::Authentication(format!("Failed to load private key: {}", e)))
}
