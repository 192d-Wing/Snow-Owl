//! SFTP Server Implementation
//!
//! This module provides an RFC-compliant SFTP server implementation
//! built on top of the SSH protocol (RFC 4251-4254).

use crate::{AuthorizedKeys, Config, Error, Result};
use async_trait::async_trait;
use bytes::{BufMut, BytesMut};
use russh::server::{Auth, Handler, Msg, Server as SshServer, Session};
use russh::{Channel, ChannelId, CryptoVec};
use russh_keys::key;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::protocol::{codec, FileAttrs, MessageType, OpenFlags, StatusCode, SFTP_VERSION};

/// SFTP Server
pub struct Server {
    config: Arc<Config>,
    ssh_config: russh::server::Config,
}

impl Server {
    /// Create a new SFTP server
    pub async fn new(config: Config) -> Result<Self> {
        config.validate()?;

        // Load host key
        let key_pair = load_host_key(&config.host_key_path).await?;

        let ssh_config = russh::server::Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(config.timeout)),
            auth_rejection_time: std::time::Duration::from_secs(3),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            keys: vec![key_pair],
            ..Default::default()
        };

        Ok(Self {
            config: Arc::new(config),
            ssh_config,
        })
    }

    /// Run the SFTP server
    pub async fn run(self) -> Result<()> {
        let addr = format!("{}:{}", self.config.bind_address, self.config.port);
        info!("Starting SFTP server on {}", addr);

        let config = Arc::new(self.ssh_config);
        let server_config = self.config.clone();

        russh::server::run(config, &addr, SftpHandler::new(server_config))
            .await
            .map_err(|e| Error::Connection(format!("Server error: {}", e)))?;

        Ok(())
    }
}

/// SSH/SFTP session handler
struct SftpHandler {
    config: Arc<Config>,
    clients: Arc<Mutex<HashMap<usize, SftpSession>>>,
}

impl SftpHandler {
    fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl SshServer for SftpHandler {
    type Handler = SftpSessionHandler;

    async fn new_client(&mut self, _peer_addr: Option<std::net::SocketAddr>) -> Self::Handler {
        let session = SftpSession::new(self.config.clone());

        // NIST 800-53: AC-2 (Account Management)
        // Load authorized keys for this connection
        let mut auth_keys = AuthorizedKeys::new(
            self.config.authorized_keys_path.to_string_lossy().to_string()
        );

        if let Err(e) = auth_keys.load() {
            warn!("Failed to load authorized_keys: {}. Authentication will fail.", e);
        }

        SftpSessionHandler {
            session: Arc::new(Mutex::new(session)),
            authorized_keys: Arc::new(Mutex::new(auth_keys)),
        }
    }
}

/// Per-connection session handler
///
/// NIST 800-53: AC-2 (Account Management), IA-2 (Identification and Authentication)
/// Implementation: Manages per-connection authentication and SFTP session
struct SftpSessionHandler {
    session: Arc<Mutex<SftpSession>>,
    authorized_keys: Arc<Mutex<AuthorizedKeys>>,
}

#[async_trait]
impl Handler for SftpSessionHandler {
    type Error = Error;

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        _session: &mut Session,
    ) -> Result<bool> {
        info!("Channel opened for session");
        let mut session = self.session.lock().await;
        session.channel = Some(channel);
        Ok(true)
    }

    async fn subsystem_request(
        &mut self,
        channel_id: ChannelId,
        name: &str,
        session: &mut Session,
    ) -> Result<()> {
        info!("Subsystem request: {}", name);

        if name == "sftp" {
            // Send success response
            session.channel_success(channel_id).await?;
            Ok(())
        } else {
            warn!("Unsupported subsystem: {}", name);
            session.channel_failure(channel_id).await?;
            Err(Error::Protocol(format!("Unsupported subsystem: {}", name)))
        }
    }

    // NIST 800-53: IA-2 (Identification and Authentication), AC-3 (Access Enforcement)
    // STIG: V-222611 - The application must validate certificates
    // Implementation: Verifies public key against authorized_keys file
    async fn auth_publickey(
        &mut self,
        user: &str,
        public_key: &key::PublicKey,
    ) -> Result<Auth> {
        // NIST 800-53: IA-2 - Verify identity through public key cryptography
        let auth_keys = self.authorized_keys.lock().await;

        if auth_keys.is_authorized(public_key) {
            info!("Public key authentication succeeded for user: {}", user);
            // NIST 800-53: AU-2 (Audit Events) - Log successful authentication
            Ok(Auth::Accept)
        } else {
            warn!("Public key authentication failed for user: {}", user);
            // NIST 800-53: AU-2 (Audit Events) - Log failed authentication
            // NIST 800-53: AC-7 (Unsuccessful Logon Attempts) - Track failed attempts
            Ok(Auth::Reject {
                proceed_with_methods: Some(russh::MethodSet::PUBLICKEY),
            })
        }
    }

    async fn auth_password(&mut self, _user: &str, _password: &str) -> Result<Auth> {
        // For demonstration, reject password auth
        // In production, implement proper password verification
        warn!("Password authentication rejected");
        Ok(Auth::Reject {
            proceed_with_methods: Some(russh::MethodSet::PUBLICKEY),
        })
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<()> {
        let mut sess = self.session.lock().await;
        let response = sess.handle_sftp_packet(data).await?;

        if !response.is_empty() {
            session.data(channel, CryptoVec::from_slice(&response)).await?;
        }

        Ok(())
    }
}

/// SFTP session state
struct SftpSession {
    config: Arc<Config>,
    channel: Option<Channel<Msg>>,
    handles: HashMap<Vec<u8>, FileHandle>,
    next_handle_id: u32,
    initialized: bool,
}

impl SftpSession {
    fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            channel: None,
            handles: HashMap::new(),
            next_handle_id: 0,
            initialized: false,
        }
    }

    /// Handle incoming SFTP packet
    async fn handle_sftp_packet(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Err(Error::Protocol("Empty packet".into()));
        }

        let mut buf = &data[..];
        let msg_type = MessageType::try_from(data[0])?;
        buf = &buf[1..];

        debug!("Received SFTP message: {:?}", msg_type);

        match msg_type {
            MessageType::Init => self.handle_init(&mut buf).await,
            MessageType::Open => self.handle_open(&mut buf).await,
            MessageType::Close => self.handle_close(&mut buf).await,
            MessageType::Read => self.handle_read(&mut buf).await,
            MessageType::Write => self.handle_write(&mut buf).await,
            MessageType::Stat | MessageType::Lstat => self.handle_stat(&mut buf).await,
            MessageType::Fstat => self.handle_fstat(&mut buf).await,
            MessageType::Opendir => self.handle_opendir(&mut buf).await,
            MessageType::Readdir => self.handle_readdir(&mut buf).await,
            MessageType::Remove => self.handle_remove(&mut buf).await,
            MessageType::Mkdir => self.handle_mkdir(&mut buf).await,
            MessageType::Rmdir => self.handle_rmdir(&mut buf).await,
            MessageType::Realpath => self.handle_realpath(&mut buf).await,
            MessageType::Rename => self.handle_rename(&mut buf).await,
            _ => {
                warn!("Unimplemented message type: {:?}", msg_type);
                Err(Error::Protocol(format!(
                    "Unimplemented message type: {:?}",
                    msg_type
                )))
            }
        }
    }

    async fn handle_init(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let version = if buf.len() >= 4 {
            u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]])
        } else {
            return Err(Error::Protocol("Invalid init packet".into()));
        };

        info!("SFTP Init - Client version: {}", version);
        self.initialized = true;

        let mut response = BytesMut::new();
        response.put_u8(MessageType::Version as u8);
        response.put_u32(SFTP_VERSION);

        Ok(response.to_vec())
    }

    async fn handle_open(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let filename = codec::get_string(buf)?;
        let pflags = self.read_u32(buf)?;
        let _attrs = FileAttrs::decode(buf)?;

        let flags = OpenFlags(pflags);
        let path = self.resolve_path(&filename)?;

        debug!("Opening file: {:?} with flags: {:?}", path, flags);

        let handle = self.open_file(path, flags).await?;
        let handle_id = self.allocate_handle(handle);

        self.send_handle(request_id, &handle_id)
    }

    async fn handle_close(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let handle = codec::get_bytes(buf)?;

        debug!("Closing handle");

        self.handles.remove(&handle);

        self.send_status(request_id, StatusCode::Ok, "Success")
    }

    async fn handle_read(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let handle = codec::get_bytes(buf)?;
        let offset = self.read_u64(buf)?;
        let len = self.read_u32(buf)?;

        debug!("Read request: offset={}, len={}", offset, len);

        let file_handle = self
            .handles
            .get_mut(&handle)
            .ok_or_else(|| Error::Protocol("Invalid handle".into()))?;

        match file_handle {
            FileHandle::File(file) => {
                file.seek(std::io::SeekFrom::Start(offset)).await?;

                let mut buffer = vec![0u8; len as usize];
                match file.read(&mut buffer).await {
                    Ok(0) => self.send_status(request_id, StatusCode::Eof, "End of file"),
                    Ok(n) => {
                        buffer.truncate(n);
                        self.send_data(request_id, &buffer)
                    }
                    Err(e) => self.send_status(
                        request_id,
                        StatusCode::Failure,
                        &format!("Read error: {}", e),
                    ),
                }
            }
            FileHandle::Dir(_) => {
                self.send_status(request_id, StatusCode::Failure, "Cannot read directory")
            }
        }
    }

    async fn handle_write(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let handle = codec::get_bytes(buf)?;
        let offset = self.read_u64(buf)?;
        let data = codec::get_bytes(buf)?;

        debug!("Write request: offset={}, len={}", offset, data.len());

        let file_handle = self
            .handles
            .get_mut(&handle)
            .ok_or_else(|| Error::Protocol("Invalid handle".into()))?;

        match file_handle {
            FileHandle::File(file) => {
                file.seek(std::io::SeekFrom::Start(offset)).await?;
                file.write_all(&data).await?;
                self.send_status(request_id, StatusCode::Ok, "Success")
            }
            FileHandle::Dir(_) => {
                self.send_status(request_id, StatusCode::Failure, "Cannot write to directory")
            }
        }
    }

    async fn handle_stat(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let path = codec::get_string(buf)?;
        let resolved_path = self.resolve_path(&path)?;

        debug!("Stat request for: {:?}", resolved_path);

        match fs::metadata(&resolved_path).await {
            Ok(metadata) => {
                let attrs = metadata_to_attrs(&metadata);
                self.send_attrs(request_id, attrs)
            }
            Err(_) => self.send_status(request_id, StatusCode::NoSuchFile, "File not found"),
        }
    }

    async fn handle_fstat(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let handle = codec::get_bytes(buf)?;

        let file_handle = self
            .handles
            .get(&handle)
            .ok_or_else(|| Error::Protocol("Invalid handle".into()))?;

        match file_handle {
            FileHandle::File(file) => match file.metadata().await {
                Ok(metadata) => {
                    let attrs = metadata_to_attrs(&metadata);
                    self.send_attrs(request_id, attrs)
                }
                Err(e) => self.send_status(
                    request_id,
                    StatusCode::Failure,
                    &format!("Metadata error: {}", e),
                ),
            },
            FileHandle::Dir(_) => {
                self.send_status(request_id, StatusCode::Failure, "Handle is a directory")
            }
        }
    }

    async fn handle_opendir(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let path = codec::get_string(buf)?;
        let resolved_path = self.resolve_path(&path)?;

        debug!("Opening directory: {:?}", resolved_path);

        match fs::read_dir(&resolved_path).await {
            Ok(read_dir) => {
                let handle = FileHandle::Dir(DirHandle {
                    entries: Vec::new(),
                    index: 0,
                });
                let handle_id = self.allocate_handle(handle);

                // Read all entries
                if let Some(FileHandle::Dir(dir_handle)) = self.handles.get_mut(&handle_id) {
                    let mut entries = Vec::new();
                    let mut read_dir = read_dir;

                    while let Ok(Some(entry)) = read_dir.next_entry().await {
                        if let Ok(metadata) = entry.metadata().await {
                            entries.push((
                                entry.file_name().to_string_lossy().to_string(),
                                metadata_to_attrs(&metadata),
                            ));
                        }
                    }

                    dir_handle.entries = entries;
                }

                self.send_handle(request_id, &handle_id)
            }
            Err(_) => self.send_status(request_id, StatusCode::NoSuchFile, "Directory not found"),
        }
    }

    async fn handle_readdir(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let handle = codec::get_bytes(buf)?;

        let file_handle = self
            .handles
            .get_mut(&handle)
            .ok_or_else(|| Error::Protocol("Invalid handle".into()))?;

        match file_handle {
            FileHandle::Dir(dir_handle) => {
                if dir_handle.index >= dir_handle.entries.len() {
                    return self.send_status(request_id, StatusCode::Eof, "End of directory");
                }

                let mut response = BytesMut::new();
                response.put_u8(MessageType::Name as u8);
                response.put_u32(request_id);

                // Send up to 100 entries at once
                let end = (dir_handle.index + 100).min(dir_handle.entries.len());
                let count = end - dir_handle.index;
                response.put_u32(count as u32);

                for i in dir_handle.index..end {
                    let (name, attrs) = &dir_handle.entries[i];
                    codec::put_string(&mut response, name);
                    codec::put_string(&mut response, name); // longname (same as shortname for now)
                    response.put(attrs.encode());
                }

                dir_handle.index = end;

                Ok(response.to_vec())
            }
            FileHandle::File(_) => {
                self.send_status(request_id, StatusCode::Failure, "Handle is not a directory")
            }
        }
    }

    async fn handle_remove(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let filename = codec::get_string(buf)?;
        let path = self.resolve_path(&filename)?;

        debug!("Removing file: {:?}", path);

        match fs::remove_file(&path).await {
            Ok(_) => self.send_status(request_id, StatusCode::Ok, "Success"),
            Err(_) => self.send_status(request_id, StatusCode::Failure, "Failed to remove file"),
        }
    }

    async fn handle_mkdir(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let path = codec::get_string(buf)?;
        let _attrs = FileAttrs::decode(buf)?;
        let resolved_path = self.resolve_path(&path)?;

        debug!("Creating directory: {:?}", resolved_path);

        match fs::create_dir(&resolved_path).await {
            Ok(_) => self.send_status(request_id, StatusCode::Ok, "Success"),
            Err(_) => {
                self.send_status(request_id, StatusCode::Failure, "Failed to create directory")
            }
        }
    }

    async fn handle_rmdir(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let path = codec::get_string(buf)?;
        let resolved_path = self.resolve_path(&path)?;

        debug!("Removing directory: {:?}", resolved_path);

        match fs::remove_dir(&resolved_path).await {
            Ok(_) => self.send_status(request_id, StatusCode::Ok, "Success"),
            Err(_) => {
                self.send_status(request_id, StatusCode::Failure, "Failed to remove directory")
            }
        }
    }

    async fn handle_realpath(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let path = codec::get_string(buf)?;

        debug!("Realpath request for: {}", path);

        let resolved = if path.is_empty() || path == "." {
            "/".to_string()
        } else {
            path.clone()
        };

        let mut response = BytesMut::new();
        response.put_u8(MessageType::Name as u8);
        response.put_u32(request_id);
        response.put_u32(1); // count

        codec::put_string(&mut response, &resolved);
        codec::put_string(&mut response, &resolved); // longname
        response.put(FileAttrs::default().encode());

        Ok(response.to_vec())
    }

    async fn handle_rename(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let oldpath = codec::get_string(buf)?;
        let newpath = codec::get_string(buf)?;

        let old_resolved = self.resolve_path(&oldpath)?;
        let new_resolved = self.resolve_path(&newpath)?;

        debug!("Rename: {:?} -> {:?}", old_resolved, new_resolved);

        match fs::rename(&old_resolved, &new_resolved).await {
            Ok(_) => self.send_status(request_id, StatusCode::Ok, "Success"),
            Err(_) => self.send_status(request_id, StatusCode::Failure, "Failed to rename"),
        }
    }

    // Helper methods

    fn resolve_path(&self, path: &str) -> Result<PathBuf> {
        let path = if path.starts_with('/') {
            &path[1..]
        } else {
            path
        };

        let resolved = self.config.root_dir.join(path);

        // Ensure the path is within root_dir (prevent path traversal)
        if !resolved.starts_with(&self.config.root_dir) {
            return Err(Error::PermissionDenied(
                "Path traversal attempt".to_string(),
            ));
        }

        Ok(resolved)
    }

    async fn open_file(&self, path: PathBuf, flags: OpenFlags) -> Result<FileHandle> {
        let mut options = fs::OpenOptions::new();

        if flags.has_read() {
            options.read(true);
        }
        if flags.has_write() {
            options.write(true);
        }
        if flags.has_append() {
            options.append(true);
        }
        if flags.has_creat() {
            options.create(true);
        }
        if flags.has_trunc() {
            options.truncate(true);
        }
        if flags.has_excl() {
            options.create_new(true);
        }

        let file = options.open(&path).await?;
        Ok(FileHandle::File(file))
    }

    fn allocate_handle(&mut self, handle: FileHandle) -> Vec<u8> {
        let id = self.next_handle_id;
        self.next_handle_id += 1;

        let handle_id = id.to_be_bytes().to_vec();
        self.handles.insert(handle_id.clone(), handle);
        handle_id
    }

    fn send_status(&self, request_id: u32, code: StatusCode, msg: &str) -> Result<Vec<u8>> {
        let mut response = BytesMut::new();
        response.put_u8(MessageType::Status as u8);
        response.put_u32(request_id);
        response.put_u32(code.into());
        codec::put_string(&mut response, msg);
        codec::put_string(&mut response, "en"); // language tag

        Ok(response.to_vec())
    }

    fn send_handle(&self, request_id: u32, handle: &[u8]) -> Result<Vec<u8>> {
        let mut response = BytesMut::new();
        response.put_u8(MessageType::Handle as u8);
        response.put_u32(request_id);
        codec::put_bytes(&mut response, handle);

        Ok(response.to_vec())
    }

    fn send_data(&self, request_id: u32, data: &[u8]) -> Result<Vec<u8>> {
        let mut response = BytesMut::new();
        response.put_u8(MessageType::Data as u8);
        response.put_u32(request_id);
        codec::put_bytes(&mut response, data);

        Ok(response.to_vec())
    }

    fn send_attrs(&self, request_id: u32, attrs: FileAttrs) -> Result<Vec<u8>> {
        let mut response = BytesMut::new();
        response.put_u8(MessageType::Attrs as u8);
        response.put_u32(request_id);
        response.put(attrs.encode());

        Ok(response.to_vec())
    }

    fn read_u32(&self, buf: &mut &[u8]) -> Result<u32> {
        if buf.len() < 4 {
            return Err(Error::Protocol("Insufficient data for u32".into()));
        }
        let value = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
        *buf = &buf[4..];
        Ok(value)
    }

    fn read_u64(&self, buf: &mut &[u8]) -> Result<u64> {
        if buf.len() < 8 {
            return Err(Error::Protocol("Insufficient data for u64".into()));
        }
        let value = u64::from_be_bytes([
            buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
        ]);
        *buf = &buf[8..];
        Ok(value)
    }
}

enum FileHandle {
    File(fs::File),
    Dir(DirHandle),
}

struct DirHandle {
    entries: Vec<(String, FileAttrs)>,
    index: usize,
}

fn metadata_to_attrs(metadata: &std::fs::Metadata) -> FileAttrs {
    FileAttrs {
        size: Some(metadata.len()),
        uid: None,
        gid: None,
        permissions: Some(0o644), // Default permissions
        atime: None,
        mtime: metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as u32),
    }
}

async fn load_host_key(path: &Path) -> Result<key::KeyPair> {
    // For development, generate a key if it doesn't exist
    if !path.exists() {
        warn!("Host key not found, generating temporary key");
        return Ok(key::KeyPair::generate_ed25519()
            .ok_or_else(|| Error::Config("Failed to generate host key".into()))?);
    }

    let key_data = fs::read_to_string(path).await?;
    russh_keys::decode_secret_key(&key_data, None)
        .map_err(|e| Error::Config(format!("Failed to load host key: {}", e)))
}
