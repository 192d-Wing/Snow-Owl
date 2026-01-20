//! SFTP Server Implementation
//!
//! This module provides an RFC-compliant SFTP server implementation
//! built on top of the SSH protocol (RFC 4251-4254).

use crate::{
    cnsa, AuthorizedKeys, Config, ConnectionTracker, ConnectionTrackerConfig, Error,
    RateLimitConfig, RateLimiter, Result,
};
use async_trait::async_trait;
use bytes::{BufMut, BytesMut};
use russh::server::{Auth, Handler, Msg, Server as SshServer, Session};
use russh::{Channel, ChannelId, CryptoVec};
use russh_keys::key;
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

use crate::protocol::{codec, FileAttrs, MessageType, OpenFlags, StatusCode, SFTP_VERSION};

/// File operation timeout (30 seconds)
///
/// NIST 800-53: AC-12 (Session Termination)
/// Implementation: Prevent operations from hanging indefinitely
const FILE_OP_TIMEOUT: Duration = Duration::from_secs(30);

/// SFTP Server
pub struct Server {
    config: Arc<Config>,
    ssh_config: russh::server::Config,
}

impl Server {
    /// Create a new SFTP server with NSA CNSA 2.0 compliant cryptography
    ///
    /// CNSS Advisory: Commercial National Security Algorithm Suite 2.0
    /// Implementation: Enforces CNSA 2.0 cipher suite for SECRET and below
    pub async fn new(config: Config) -> Result<Self> {
        config.validate()?;

        // Load host key
        let key_pair = load_host_key(&config.host_key_path).await?;

        // NSA CNSA 2.0: Configure cryptographic algorithms
        // Only CNSA 2.0 compliant algorithms are enabled
        let mut ssh_config = russh::server::Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(config.timeout)),
            auth_rejection_time: std::time::Duration::from_secs(3),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            keys: vec![key_pair],
            ..Default::default()
        };

        // CNSA 2.0: Configure only approved algorithms
        ssh_config.preferred = russh::Preferred {
            kex: cnsa::CNSA_KEX_ALGORITHMS,
            key: cnsa::CNSA_HOST_KEY_ALGORITHMS,
            cipher: cnsa::CNSA_CIPHERS,
            mac: cnsa::CNSA_MAC_ALGORITHMS,
            ..Default::default()
        };

        info!(
            event = "cnsa_compliance",
            kex_algorithms = ?cnsa::CNSA_KEX_ALGORITHMS,
            ciphers = ?cnsa::CNSA_CIPHERS,
            mac_algorithms = ?cnsa::CNSA_MAC_ALGORITHMS,
            host_key_algorithms = ?cnsa::CNSA_HOST_KEY_ALGORITHMS,
            "NSA CNSA 2.0 cipher suite enforced"
        );

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
///
/// NIST 800-53: AC-7 (Unsuccessful Logon Attempts), AC-10 (Concurrent Session Control), AC-12 (Session Termination)
/// STIG: V-222601 (Session termination)
/// Implementation: Manages rate limiting and connection limits per user
struct SftpHandler {
    config: Arc<Config>,
    clients: Arc<Mutex<HashMap<usize, SftpSession>>>,
    rate_limiter: Arc<RateLimiter>,
    connection_tracker: Arc<ConnectionTracker>,
}

impl SftpHandler {
    fn new(config: Arc<Config>) -> Self {
        // NIST 800-53: AC-7 - Initialize rate limiter
        let rate_limit_config = RateLimitConfig {
            max_attempts: config.max_auth_attempts,
            window_secs: config.rate_limit_window_secs,
            lockout_duration_secs: config.lockout_duration_secs,
        };

        // NIST 800-53: AC-10 - Initialize connection tracker
        let connection_tracker_config = ConnectionTrackerConfig {
            max_connections_per_user: config.max_connections_per_user,
        };

        Self {
            config,
            clients: Arc::new(Mutex::new(HashMap::new())),
            rate_limiter: Arc::new(RateLimiter::new(rate_limit_config)),
            connection_tracker: Arc::new(ConnectionTracker::new(connection_tracker_config)),
        }
    }
}

#[async_trait]
impl SshServer for SftpHandler {
    type Handler = SftpSessionHandler;

    async fn new_client(&mut self, peer_addr: Option<std::net::SocketAddr>) -> Self::Handler {
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
            rate_limiter: self.rate_limiter.clone(),
            connection_tracker: self.connection_tracker.clone(),
            peer_addr: peer_addr.map(|addr| addr.ip()),
            username: Arc::new(Mutex::new(None)),
            connection_id: Arc::new(Mutex::new(None)),
        }
    }
}

/// Per-connection session handler
///
/// NIST 800-53: AC-2 (Account Management), IA-2 (Identification and Authentication), AC-7 (Unsuccessful Logon Attempts), AC-10 (Concurrent Session Control)
/// STIG: V-222601 (Session termination)
/// Implementation: Manages per-connection authentication and SFTP session with rate limiting and connection tracking
struct SftpSessionHandler {
    session: Arc<Mutex<SftpSession>>,
    authorized_keys: Arc<Mutex<AuthorizedKeys>>,
    rate_limiter: Arc<RateLimiter>,
    connection_tracker: Arc<ConnectionTracker>,
    peer_addr: Option<IpAddr>,
    username: Arc<Mutex<Option<String>>>,
    connection_id: Arc<Mutex<Option<usize>>>,
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

    // NIST 800-53: IA-2 (Identification and Authentication), AC-3 (Access Enforcement), AC-7 (Unsuccessful Logon Attempts), AC-10 (Concurrent Session Control)
    // STIG: V-222611 - The application must validate certificates
    // STIG: V-222578 - Implement replay-resistant authentication mechanisms
    // STIG: V-222601 - Session termination and concurrent session control
    // Implementation: Verifies public key against authorized_keys file with rate limiting and connection limits
    async fn auth_publickey(
        &mut self,
        user: &str,
        public_key: &key::PublicKey,
    ) -> Result<Auth> {
        // NIST 800-53: AC-7 - Check rate limit before attempting authentication
        if let Some(ip) = self.peer_addr {
            if !self.rate_limiter.check_allowed(ip).await {
                warn!(
                    "Rate limit exceeded for IP {}, rejecting authentication for user: {}",
                    ip, user
                );
                // NIST 800-53: AU-2 (Audit Events) - Log rate limited attempt
                return Ok(Auth::Reject {
                    proceed_with_methods: None, // No other methods allowed when rate limited
                });
            }
        }

        // NIST 800-53: IA-2 - Verify identity through public key cryptography
        let auth_keys = self.authorized_keys.lock().await;

        if auth_keys.is_authorized(public_key) {
            // NIST 800-53: AC-10 - Check concurrent session limit before accepting
            if !self.connection_tracker.can_connect(user).await {
                warn!(
                    "User '{}' exceeded maximum concurrent connections, rejecting authentication",
                    user
                );
                // NIST 800-53: AU-2 (Audit Events) - Log connection limit rejection
                return Ok(Auth::Reject {
                    proceed_with_methods: None, // Reject due to connection limit
                });
            }

            info!("Public key authentication succeeded for user: {}", user);
            // NIST 800-53: AU-2 (Audit Events) - Log successful authentication

            // NIST 800-53: AC-7 - Clear failed attempts on success
            if let Some(ip) = self.peer_addr {
                self.rate_limiter.record_success(ip).await;
            }

            // NIST 800-53: AC-10 - Register connection for user
            if let Some(conn_id) = self
                .connection_tracker
                .register_connection(user.to_string())
                .await
            {
                let mut username = self.username.lock().await;
                *username = Some(user.to_string());

                let mut connection_id = self.connection_id.lock().await;
                *connection_id = Some(conn_id);

                Ok(Auth::Accept)
            } else {
                warn!(
                    "Failed to register connection for user '{}' (connection limit reached)",
                    user
                );
                Ok(Auth::Reject {
                    proceed_with_methods: None,
                })
            }
        } else {
            warn!("Public key authentication failed for user: {}", user);
            // NIST 800-53: AU-2 (Audit Events) - Log failed authentication
            // NIST 800-53: AC-7 (Unsuccessful Logon Attempts) - Track failed attempts

            if let Some(ip) = self.peer_addr {
                self.rate_limiter.record_failure(ip).await;
            }

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

    /// Handle SFTP data
    ///
    /// NIST 800-53: SI-11 (Error Handling), SC-8 (Transmission Confidentiality)
    /// STIG: V-222566
    /// Implementation: Robust handling of SFTP packets with error recovery
    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<()> {
        let mut sess = self.session.lock().await;

        // NIST 800-53: SI-11 - Handle packet processing errors gracefully
        let response = match sess.handle_sftp_packet(data).await {
            Ok(resp) => resp,
            Err(e) => {
                // NIST 800-53: AU-2 - Log error
                error!("SFTP packet handling error: {}", e);

                // NIST 800-53: AU-2 - Log security events
                if e.is_security_event() {
                    warn!("Security event during SFTP operation: {}", e);
                }

                // Try to extract request ID for error response
                // If we can't send an error response, the error will propagate
                return Err(e);
            }
        };

        if !response.is_empty() {
            // NIST 800-53: SC-8, SI-11 - Handle channel write errors (connection drops)
            if let Err(e) = session.data(channel, CryptoVec::from_slice(&response)).await {
                error!("Failed to send response, channel may be closed: {}", e);
                return Err(Error::channel_closed(format!(
                    "Failed to send response: {}",
                    e
                )));
            }
        }

        Ok(())
    }

    // NIST 800-53: AC-12 (Session Termination), AC-10 (Concurrent Session Control)
    // STIG: V-222601 - Session termination
    // Implementation: Clean up connection tracking on session end
    async fn finished(&mut self, _session: &mut Session) -> Result<()> {
        // Unregister connection when session finishes
        let username = self.username.lock().await;
        let connection_id = self.connection_id.lock().await;

        if let (Some(user), Some(conn_id)) = (username.as_ref(), *connection_id) {
            info!(
                "Session finished for user '{}', unregistering connection {}",
                user, conn_id
            );
            self.connection_tracker
                .unregister_connection(user, conn_id)
                .await;
        }

        Ok(())
    }
}

/// SFTP session state
///
/// NIST 800-53: SI-11 (Error Handling), AC-12 (Session Termination)
/// STIG: V-222601
/// Implementation: Session state with automatic resource cleanup
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
}

impl Drop for SftpSession {
    /// NIST 800-53: SI-11, AC-12 - Clean up all file handles on session end
    /// STIG: V-222601
    /// Implementation: Ensures all file handles are closed when session terminates
    fn drop(&mut self) {
        let handle_count = self.handles.len();
        if handle_count > 0 {
            info!("Cleaning up {} open file handles on session end", handle_count);
            self.handles.clear();
        }
    }
}

impl SftpSession {
    /// Handle incoming SFTP packet
    ///
    /// NIST 800-53: SI-11 (Error Handling)
    /// STIG: V-222566
    /// Implementation: Robust error handling for all SFTP operations
    async fn handle_sftp_packet(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            error!("Received empty SFTP packet");
            return Err(Error::Protocol("Empty packet".into()));
        }

        let mut buf = &data[..];
        let msg_type = MessageType::try_from(data[0])?;
        buf = &buf[1..];

        debug!("Received SFTP message: {:?}", msg_type);

        // Check if session is initialized (except for INIT message)
        if !self.initialized && msg_type != MessageType::Init {
            error!("Received {:?} message before initialization", msg_type);
            return Err(Error::Protocol("Session not initialized".into()));
        }

        match msg_type {
            MessageType::Init => self.handle_init(&mut buf).await,
            MessageType::Open => self.handle_open(&mut buf).await,
            MessageType::Close => self.handle_close(&mut buf).await,
            MessageType::Read => self.handle_read(&mut buf).await,
            MessageType::Write => self.handle_write(&mut buf).await,
            MessageType::Stat | MessageType::Lstat => self.handle_stat(&mut buf).await,
            MessageType::Fstat => self.handle_fstat(&mut buf).await,
            MessageType::Setstat => self.handle_setstat(&mut buf).await,
            MessageType::Fsetstat => self.handle_fsetstat(&mut buf).await,
            MessageType::Opendir => self.handle_opendir(&mut buf).await,
            MessageType::Readdir => self.handle_readdir(&mut buf).await,
            MessageType::Remove => self.handle_remove(&mut buf).await,
            MessageType::Mkdir => self.handle_mkdir(&mut buf).await,
            MessageType::Rmdir => self.handle_rmdir(&mut buf).await,
            MessageType::Realpath => self.handle_realpath(&mut buf).await,
            MessageType::Rename => self.handle_rename(&mut buf).await,
            MessageType::Readlink => self.handle_readlink(&mut buf).await,
            MessageType::Symlink => self.handle_symlink(&mut buf).await,
            _ => {
                warn!("Unimplemented message type: {:?}", msg_type);
                Err(Error::NotSupported(format!(
                    "Message type {:?} is not supported",
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

    /// Open file
    ///
    /// NIST 800-53: SI-11 (Error Handling), AC-3 (Access Enforcement)
    /// STIG: V-222566, V-222596
    /// Implementation: Secure file opening with validation and resource tracking
    async fn handle_open(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let filename = codec::get_string(buf)?;
        let pflags = self.read_u32(buf)?;
        let _attrs = FileAttrs::decode(buf)?;

        let flags = OpenFlags(pflags);

        // NIST 800-53: AC-3, SI-10 - Validate and resolve path
        let path = match self.resolve_path(&filename) {
            Ok(p) => p,
            Err(e) => {
                // NIST 800-53: AU-2 - Log security event
                if e.is_security_event() {
                    warn!("Security event during open: {} - {}", filename, e);
                }
                return Ok(self.send_status_error(request_id, &e)?);
            }
        };

        debug!("Opening file: {:?} with flags: {:?}", path, flags);

        // NIST 800-53: SI-11 - Check for resource exhaustion
        if self.handles.len() >= 1024 {
            warn!("Maximum file handles reached (1024)");
            return Ok(self.send_status_error(
                request_id,
                &Error::resource_exhaustion("Too many open file handles"),
            )?);
        }

        // NIST 800-53: SI-11 - Handle file opening errors
        let handle = match self.open_file(path.clone(), flags).await {
            Ok(h) => h,
            Err(e) => {
                debug!("Failed to open file {:?}: {}", path, e);
                let error = match &e {
                    Error::Io(io_err) => {
                        if io_err.kind() == std::io::ErrorKind::NotFound {
                            Error::FileNotFound(format!("File not found: {}", filename))
                        } else if io_err.kind() == std::io::ErrorKind::PermissionDenied {
                            Error::PermissionDenied(format!("Access denied: {}", filename))
                        } else {
                            e
                        }
                    }
                    _ => e,
                };
                return Ok(self.send_status_error(request_id, &error)?);
            }
        };

        let handle_id = self.allocate_handle(handle);

        self.send_handle(request_id, &handle_id)
    }

    /// Close file or directory handle
    ///
    /// NIST 800-53: SI-11 (Error Handling)
    /// Implementation: Proper cleanup of file handles with error checking
    async fn handle_close(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let handle = codec::get_bytes(buf)?;

        debug!("Closing handle");

        // NIST 800-53: SI-11 - Validate handle exists before closing
        if !self.handles.contains_key(&handle) {
            warn!("Attempt to close invalid handle");
            return Ok(self.send_status_error(
                request_id,
                &Error::invalid_handle("Handle does not exist"),
            )?);
        }

        // Remove handle (Drop trait will clean up resources)
        self.handles.remove(&handle);

        self.send_status(request_id, StatusCode::Ok, "Success")
    }

    /// Read from file handle
    ///
    /// NIST 800-53: SI-11 (Error Handling)
    /// Implementation: Safe file reading with proper error handling
    async fn handle_read(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let handle = codec::get_bytes(buf)?;
        let offset = self.read_u64(buf)?;
        let len = self.read_u32(buf)?;

        debug!("Read request: offset={}, len={}", offset, len);

        // NIST 800-53: SI-11 - Validate handle
        let file_handle = self.handles.get_mut(&handle).ok_or_else(|| {
            warn!("Read attempt with invalid handle");
            Error::invalid_handle("Handle does not exist or is closed")
        })?;

        match file_handle {
            FileHandle::File(file, _path) => {
                // NIST 800-53: SI-11 - Handle seek errors
                if let Err(e) = file.seek(std::io::SeekFrom::Start(offset)).await {
                    error!("Seek error at offset {}: {}", offset, e);
                    return Ok(self.send_status_error(request_id, &Error::Io(e))?);
                }

                let mut buffer = vec![0u8; len as usize];

                // NIST 800-53: AC-12 - Timeout protection for read operations
                let read_result = timeout(FILE_OP_TIMEOUT, file.read(&mut buffer)).await;

                match read_result {
                    Ok(Ok(0)) => self.send_status(request_id, StatusCode::Eof, "End of file"),
                    Ok(Ok(n)) => {
                        buffer.truncate(n);
                        self.send_data(request_id, &buffer)
                    }
                    Ok(Err(e)) => {
                        error!("Read error: {}", e);
                        Ok(self.send_status_error(request_id, &Error::Io(e))?)
                    }
                    Err(_) => {
                        error!("Read operation timed out after {} seconds", FILE_OP_TIMEOUT.as_secs());
                        Ok(self.send_status_error(
                            request_id,
                            &Error::timeout(format!("Read operation timed out")),
                        )?)
                    }
                }
            }
            FileHandle::Dir(_) => {
                warn!("Attempt to read from directory handle");
                Ok(self.send_status_error(
                    request_id,
                    &Error::InvalidHandle("Cannot read from directory handle".into()),
                )?)
            }
        }
    }

    /// Write to file handle
    ///
    /// NIST 800-53: SI-11 (Error Handling)
    /// Implementation: Safe file writing with proper error handling
    async fn handle_write(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let handle = codec::get_bytes(buf)?;
        let offset = self.read_u64(buf)?;
        let data = codec::get_bytes(buf)?;

        debug!("Write request: offset={}, len={}", offset, data.len());

        // NIST 800-53: SI-11 - Validate handle
        let file_handle = self.handles.get_mut(&handle).ok_or_else(|| {
            warn!("Write attempt with invalid handle");
            Error::invalid_handle("Handle does not exist or is closed")
        })?;

        match file_handle {
            FileHandle::File(file, _path) => {
                // NIST 800-53: SI-11 - Handle seek errors
                if let Err(e) = file.seek(std::io::SeekFrom::Start(offset)).await {
                    error!("Seek error at offset {}: {}", offset, e);
                    return Ok(self.send_status_error(request_id, &Error::Io(e))?);
                }

                // NIST 800-53: AC-12 - Timeout protection for write operations
                let write_result = timeout(FILE_OP_TIMEOUT, file.write_all(&data)).await;

                match write_result {
                    Ok(Ok(())) => self.send_status(request_id, StatusCode::Ok, "Success"),
                    Ok(Err(e)) => {
                        error!("Write error: {}", e);
                        Ok(self.send_status_error(request_id, &Error::Io(e))?)
                    }
                    Err(_) => {
                        error!("Write operation timed out after {} seconds", FILE_OP_TIMEOUT.as_secs());
                        Ok(self.send_status_error(
                            request_id,
                            &Error::timeout(format!("Write operation timed out")),
                        )?)
                    }
                }
            }
            FileHandle::Dir(_) => {
                warn!("Attempt to write to directory handle");
                Ok(self.send_status_error(
                    request_id,
                    &Error::InvalidHandle("Cannot write to directory handle".into()),
                )?)
            }
        }
    }

    /// Get file/directory attributes
    ///
    /// NIST 800-53: SI-11 (Error Handling), AC-3 (Access Enforcement)
    /// STIG: V-222566, V-222596
    /// Implementation: Secure attribute retrieval with proper error handling
    async fn handle_stat(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let path = codec::get_string(buf)?;

        // NIST 800-53: AC-3, SI-10 - Validate and resolve path
        let resolved_path = match self.resolve_path(&path) {
            Ok(p) => p,
            Err(e) => {
                // NIST 800-53: AU-2 - Log security event
                if e.is_security_event() {
                    warn!("Security event during stat: {} - {}", path, e);
                }
                return Ok(self.send_status_error(request_id, &e)?);
            }
        };

        debug!("Stat request for: {:?}", resolved_path);

        // NIST 800-53: AC-12 - Timeout protection for metadata operations
        let metadata_result = timeout(FILE_OP_TIMEOUT, fs::metadata(&resolved_path)).await;

        match metadata_result {
            Ok(Ok(metadata)) => {
                let attrs = metadata_to_attrs(&metadata);
                self.send_attrs(request_id, attrs)
            }
            Ok(Err(e)) => {
                debug!("Stat failed for {:?}: {}", resolved_path, e);
                Ok(self.send_status_error(
                    request_id,
                    &Error::FileNotFound(format!("File not found: {}", path)),
                )?)
            }
            Err(_) => {
                error!("Stat operation timed out after {} seconds", FILE_OP_TIMEOUT.as_secs());
                Ok(self.send_status_error(
                    request_id,
                    &Error::timeout("Stat operation timed out"),
                )?)
            }
        }
    }

    /// Get attributes for file handle
    ///
    /// NIST 800-53: SI-11 (Error Handling)
    /// Implementation: Safe attribute retrieval with handle validation
    async fn handle_fstat(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let handle = codec::get_bytes(buf)?;

        // NIST 800-53: SI-11 - Validate handle
        let file_handle = self.handles.get(&handle).ok_or_else(|| {
            warn!("Fstat attempt with invalid handle");
            Error::invalid_handle("Handle does not exist or is closed")
        })?;

        match file_handle {
            FileHandle::File(file, _path) => match file.metadata().await {
                Ok(metadata) => {
                    let attrs = metadata_to_attrs(&metadata);
                    self.send_attrs(request_id, attrs)
                }
                Err(e) => {
                    error!("Metadata error: {}", e);
                    Ok(self.send_status_error(request_id, &Error::Io(e))?)
                }
            },
            FileHandle::Dir(_) => {
                warn!("Attempt to fstat directory handle");
                Ok(self.send_status_error(
                    request_id,
                    &Error::InvalidHandle("Cannot fstat directory handle".into()),
                )?)
            }
        }
    }

    /// Set file/directory attributes
    ///
    /// NIST 800-53: SI-11 (Error Handling), AC-3 (Access Enforcement)
    /// STIG: V-222566, V-222596
    /// Implementation: Secure attribute modification with validation
    async fn handle_setstat(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let path = codec::get_string(buf)?;
        let attrs = FileAttrs::decode(buf)?;

        // NIST 800-53: AC-3, SI-10 - Validate and resolve path
        let resolved_path = match self.resolve_path(&path) {
            Ok(p) => p,
            Err(e) => {
                // NIST 800-53: AU-2 - Log security event
                if e.is_security_event() {
                    warn!("Security event during setstat: {} - {}", path, e);
                }
                return Ok(self.send_status_error(request_id, &e)?);
            }
        };

        debug!("Setstat request for: {:?}", resolved_path);

        // Apply attributes
        if let Err(e) = self.apply_file_attrs(&resolved_path, &attrs).await {
            debug!("Failed to set attributes for {:?}: {}", resolved_path, e);
            return Ok(self.send_status_error(request_id, &e)?);
        }

        self.send_status(request_id, StatusCode::Ok, "Success")
    }

    /// Set attributes for file handle
    ///
    /// NIST 800-53: SI-11 (Error Handling), AC-3 (Access Enforcement)
    /// STIG: V-222566, V-222596
    /// Implementation: Secure attribute modification by handle with validation
    async fn handle_fsetstat(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let handle = codec::get_bytes(buf)?;
        let attrs = FileAttrs::decode(buf)?;

        debug!("Fsetstat request");

        // NIST 800-53: SI-11 - Validate handle
        let file_handle = self.handles.get(&handle).ok_or_else(|| {
            warn!("Fsetstat attempt with invalid handle");
            Error::invalid_handle("Handle does not exist or is closed")
        })?;

        // Get the file path from the handle
        let path = match file_handle {
            FileHandle::File(_file, path) => path.clone(),
            FileHandle::Dir(_) => {
                warn!("Attempt to fsetstat directory handle");
                return Ok(self.send_status_error(
                    request_id,
                    &Error::InvalidHandle("Cannot fsetstat directory handle".into()),
                )?);
            }
        };

        // Apply attributes
        if let Err(e) = self.apply_file_attrs(&path, &attrs).await {
            debug!("Failed to set attributes for {:?}: {}", path, e);
            return Ok(self.send_status_error(request_id, &e)?);
        }

        self.send_status(request_id, StatusCode::Ok, "Success")
    }

    /// Open directory for reading
    ///
    /// NIST 800-53: SI-11 (Error Handling), AC-3 (Access Enforcement)
    /// STIG: V-222566, V-222596
    /// Implementation: Secure directory opening with validation
    async fn handle_opendir(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let path = codec::get_string(buf)?;

        // NIST 800-53: AC-3, SI-10 - Validate and resolve path
        let resolved_path = match self.resolve_path(&path) {
            Ok(p) => p,
            Err(e) => {
                // NIST 800-53: AU-2 - Log security event
                if e.is_security_event() {
                    warn!("Security event during opendir: {} - {}", path, e);
                }
                return Ok(self.send_status_error(request_id, &e)?);
            }
        };

        debug!("Opening directory: {:?}", resolved_path);

        // NIST 800-53: AC-12 - Timeout protection for directory operations
        let read_dir_result = timeout(FILE_OP_TIMEOUT, fs::read_dir(&resolved_path)).await;

        match read_dir_result {
            Ok(result) => match result {
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
                Err(e) => {
                    debug!("Failed to open directory {:?}: {}", resolved_path, e);
                    let error = if e.kind() == std::io::ErrorKind::NotFound {
                        Error::FileNotFound(format!("Directory not found: {}", path))
                    } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                        Error::PermissionDenied(format!("Access denied: {}", path))
                    } else {
                        Error::Io(e)
                    };
                    Ok(self.send_status_error(request_id, &error)?)
                }
            },
            Err(_) => {
                error!("Opendir operation timed out after {} seconds", FILE_OP_TIMEOUT.as_secs());
                Ok(self.send_status_error(
                    request_id,
                    &Error::timeout("Directory operation timed out"),
                )?)
            }
        }
    }

    /// Read directory entries
    ///
    /// NIST 800-53: SI-11 (Error Handling)
    /// Implementation: Safe directory reading with handle validation
    async fn handle_readdir(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let handle = codec::get_bytes(buf)?;

        // NIST 800-53: SI-11 - Validate handle
        let file_handle = self.handles.get_mut(&handle).ok_or_else(|| {
            warn!("Readdir attempt with invalid handle");
            Error::invalid_handle("Handle does not exist or is closed")
        })?;

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
            FileHandle::File(_, _) => {
                warn!("Attempt to readdir from file handle");
                Ok(self.send_status_error(
                    request_id,
                    &Error::InvalidHandle("Cannot readdir from file handle".into()),
                )?)
            }
        }
    }

    /// Remove file
    ///
    /// NIST 800-53: SI-11 (Error Handling), AC-3 (Access Enforcement)
    /// STIG: V-222566, V-222596
    /// Implementation: Secure file removal with validation and error handling
    async fn handle_remove(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let filename = codec::get_string(buf)?;

        // NIST 800-53: AC-3, SI-10 - Validate and resolve path
        let path = match self.resolve_path(&filename) {
            Ok(p) => p,
            Err(e) => {
                // NIST 800-53: AU-2 - Log security event
                if e.is_security_event() {
                    warn!("Security event during remove: {} - {}", filename, e);
                }
                return Ok(self.send_status_error(request_id, &e)?);
            }
        };

        debug!("Removing file: {:?}", path);

        // NIST 800-53: AC-12 - Timeout protection for file removal
        let remove_result = timeout(FILE_OP_TIMEOUT, fs::remove_file(&path)).await;

        match remove_result {
            Ok(result) => match result {
                Ok(_) => {
                    info!("File removed: {:?}", path);
                    self.send_status(request_id, StatusCode::Ok, "Success")
                }
                Err(e) => {
                    debug!("Failed to remove file {:?}: {}", path, e);
                    let error = if e.kind() == std::io::ErrorKind::NotFound {
                        Error::FileNotFound(format!("File not found: {}", filename))
                    } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                        Error::PermissionDenied(format!("Access denied: {}", filename))
                    } else {
                        Error::Io(e)
                    };
                    Ok(self.send_status_error(request_id, &error)?)
                }
            },
            Err(_) => {
                error!("Remove operation timed out after {} seconds", FILE_OP_TIMEOUT.as_secs());
                Ok(self.send_status_error(
                    request_id,
                    &Error::timeout("Remove operation timed out"),
                )?)
            }
        }
    }

    /// Create directory
    ///
    /// NIST 800-53: SI-11 (Error Handling), AC-3 (Access Enforcement)
    /// STIG: V-222566, V-222596
    /// Implementation: Secure directory creation with validation and error handling
    async fn handle_mkdir(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let path = codec::get_string(buf)?;
        let _attrs = FileAttrs::decode(buf)?;

        // NIST 800-53: AC-3, SI-10 - Validate and resolve path
        let resolved_path = match self.resolve_path(&path) {
            Ok(p) => p,
            Err(e) => {
                // NIST 800-53: AU-2 - Log security event
                if e.is_security_event() {
                    warn!("Security event during mkdir: {} - {}", path, e);
                }
                return Ok(self.send_status_error(request_id, &e)?);
            }
        };

        debug!("Creating directory: {:?}", resolved_path);

        // NIST 800-53: AC-12 - Timeout protection for directory creation
        let mkdir_result = timeout(FILE_OP_TIMEOUT, fs::create_dir(&resolved_path)).await;

        match mkdir_result {
            Ok(result) => match result {
                Ok(_) => {
                    info!("Directory created: {:?}", resolved_path);
                    self.send_status(request_id, StatusCode::Ok, "Success")
                }
                Err(e) => {
                    debug!("Failed to create directory {:?}: {}", resolved_path, e);
                    let error = if e.kind() == std::io::ErrorKind::PermissionDenied {
                        Error::PermissionDenied(format!("Access denied: {}", path))
                    } else if e.kind() == std::io::ErrorKind::AlreadyExists {
                        Error::Other(format!("Directory already exists: {}", path))
                    } else {
                        Error::Io(e)
                    };
                    Ok(self.send_status_error(request_id, &error)?)
                }
            },
            Err(_) => {
                error!("Mkdir operation timed out after {} seconds", FILE_OP_TIMEOUT.as_secs());
                Ok(self.send_status_error(
                    request_id,
                    &Error::timeout("Directory creation timed out"),
                )?)
            }
        }
    }

    /// Remove directory
    ///
    /// NIST 800-53: SI-11 (Error Handling), AC-3 (Access Enforcement)
    /// STIG: V-222566, V-222596
    /// Implementation: Secure directory removal with validation and error handling
    async fn handle_rmdir(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let path = codec::get_string(buf)?;

        // NIST 800-53: AC-3, SI-10 - Validate and resolve path
        let resolved_path = match self.resolve_path(&path) {
            Ok(p) => p,
            Err(e) => {
                // NIST 800-53: AU-2 - Log security event
                if e.is_security_event() {
                    warn!("Security event during rmdir: {} - {}", path, e);
                }
                return Ok(self.send_status_error(request_id, &e)?);
            }
        };

        debug!("Removing directory: {:?}", resolved_path);

        // NIST 800-53: AC-12 - Timeout protection for directory removal
        let rmdir_result = timeout(FILE_OP_TIMEOUT, fs::remove_dir(&resolved_path)).await;

        match rmdir_result {
            Ok(result) => match result {
                Ok(_) => {
                    info!("Directory removed: {:?}", resolved_path);
                    self.send_status(request_id, StatusCode::Ok, "Success")
                }
                Err(e) => {
                    debug!("Failed to remove directory {:?}: {}", resolved_path, e);
                    let error = if e.kind() == std::io::ErrorKind::NotFound {
                        Error::FileNotFound(format!("Directory not found: {}", path))
                    } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                        Error::PermissionDenied(format!("Access denied: {}", path))
                    } else {
                        Error::Io(e)
                    };
                    Ok(self.send_status_error(request_id, &error)?)
                }
            },
            Err(_) => {
                error!("Rmdir operation timed out after {} seconds", FILE_OP_TIMEOUT.as_secs());
                Ok(self.send_status_error(
                    request_id,
                    &Error::timeout("Directory removal timed out"),
                )?)
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

    /// Rename file or directory
    ///
    /// NIST 800-53: SI-11 (Error Handling), AC-3 (Access Enforcement)
    /// STIG: V-222566, V-222596
    /// Implementation: Secure rename with validation and error handling
    async fn handle_rename(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let oldpath = codec::get_string(buf)?;
        let newpath = codec::get_string(buf)?;

        // NIST 800-53: AC-3, SI-10 - Validate and resolve both paths
        let old_resolved = match self.resolve_path(&oldpath) {
            Ok(p) => p,
            Err(e) => {
                // NIST 800-53: AU-2 - Log security event
                if e.is_security_event() {
                    warn!("Security event during rename (old path): {} - {}", oldpath, e);
                }
                return Ok(self.send_status_error(request_id, &e)?);
            }
        };

        let new_resolved = match self.resolve_path(&newpath) {
            Ok(p) => p,
            Err(e) => {
                // NIST 800-53: AU-2 - Log security event
                if e.is_security_event() {
                    warn!("Security event during rename (new path): {} - {}", newpath, e);
                }
                return Ok(self.send_status_error(request_id, &e)?);
            }
        };

        debug!("Rename: {:?} -> {:?}", old_resolved, new_resolved);

        // NIST 800-53: AC-12 - Timeout protection for rename operations
        let rename_result = timeout(FILE_OP_TIMEOUT, fs::rename(&old_resolved, &new_resolved)).await;

        match rename_result {
            Ok(result) => match result {
                Ok(_) => {
                    info!("Renamed {:?} to {:?}", old_resolved, new_resolved);
                    self.send_status(request_id, StatusCode::Ok, "Success")
                }
                Err(e) => {
                    debug!("Failed to rename {:?} to {:?}: {}", old_resolved, new_resolved, e);
                    let error = if e.kind() == std::io::ErrorKind::NotFound {
                        Error::FileNotFound(format!("Source not found: {}", oldpath))
                    } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                        Error::PermissionDenied(format!("Access denied"))
                    } else {
                        Error::Io(e)
                    };
                    Ok(self.send_status_error(request_id, &error)?)
                }
            },
            Err(_) => {
                error!("Rename operation timed out after {} seconds", FILE_OP_TIMEOUT.as_secs());
                Ok(self.send_status_error(
                    request_id,
                    &Error::timeout("Rename operation timed out"),
                )?)
            }
        }
    }

    /// Read symbolic link target
    ///
    /// NIST 800-53: SI-11 (Error Handling), AC-3 (Access Enforcement)
    /// STIG: V-222566, V-222596
    /// Implementation: Secure symlink reading with validation
    #[cfg(unix)]
    async fn handle_readlink(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let path = codec::get_string(buf)?;

        // NIST 800-53: AC-3, SI-10 - Validate and resolve path
        let resolved_path = match self.resolve_path(&path) {
            Ok(p) => p,
            Err(e) => {
                // NIST 800-53: AU-2 - Log security event
                if e.is_security_event() {
                    warn!("Security event during readlink: {} - {}", path, e);
                }
                return Ok(self.send_status_error(request_id, &e)?);
            }
        };

        debug!("Readlink request for: {:?}", resolved_path);

        // NIST 800-53: AC-12 - Timeout protection for readlink operation
        let readlink_result = timeout(FILE_OP_TIMEOUT, fs::read_link(&resolved_path)).await;

        match readlink_result {
            Ok(result) => match result {
                Ok(target) => {
                    // NIST 800-53: AC-3 - Security check: ensure target is within root
                    // Convert target to string for response
                    let target_str = target.to_string_lossy().to_string();

                    // Check if the symlink target tries to escape root directory
                    let absolute_target = if target.is_absolute() {
                        target.clone()
                    } else {
                        // Resolve relative symlink against the symlink's parent directory
                        if let Some(parent) = resolved_path.parent() {
                            parent.join(&target)
                        } else {
                            target.clone()
                        }
                    };

                    // Security check: warn if symlink points outside root
                    if !absolute_target.starts_with(&self.config.root_dir) {
                        warn!(
                            "Symlink {:?} points outside root directory to {:?}",
                            resolved_path, absolute_target
                        );
                        // We still return the target but log the security concern
                    }

                    info!("Symlink {:?} -> {:?}", resolved_path, target);

                    let mut response = BytesMut::new();
                    response.put_u8(MessageType::Name as u8);
                    response.put_u32(request_id);
                    response.put_u32(1); // count

                    codec::put_string(&mut response, &target_str);
                    codec::put_string(&mut response, &target_str); // longname
                    response.put(FileAttrs::default().encode());

                    Ok(response.to_vec())
                }
                Err(e) => {
                    debug!("Failed to read symlink {:?}: {}", resolved_path, e);
                    let error = if e.kind() == std::io::ErrorKind::NotFound {
                        Error::FileNotFound(format!("Symlink not found: {}", path))
                    } else if e.kind() == std::io::ErrorKind::InvalidInput {
                        Error::Other(format!("Not a symlink: {}", path))
                    } else {
                        Error::Io(e)
                    };
                    Ok(self.send_status_error(request_id, &error)?)
                }
            },
            Err(_) => {
                error!("Readlink operation timed out after {} seconds", FILE_OP_TIMEOUT.as_secs());
                Ok(self.send_status_error(
                    request_id,
                    &Error::timeout("Readlink operation timed out"),
                )?)
            }
        }
    }

    /// Read symbolic link target (non-Unix fallback)
    #[cfg(not(unix))]
    async fn handle_readlink(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let _path = codec::get_string(buf)?;

        warn!("READLINK not supported on this platform");
        Ok(self.send_status_error(
            request_id,
            &Error::NotSupported("READLINK not supported on this platform".into()),
        )?)
    }

    /// Create symbolic link
    ///
    /// NIST 800-53: SI-11 (Error Handling), AC-3 (Access Enforcement)
    /// STIG: V-222566, V-222596
    /// Implementation: Secure symlink creation with validation
    #[cfg(unix)]
    async fn handle_symlink(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let linkpath = codec::get_string(buf)?;
        let targetpath = codec::get_string(buf)?;

        // NIST 800-53: AC-3, SI-10 - Validate linkpath (where symlink will be created)
        let resolved_linkpath = match self.resolve_path(&linkpath) {
            Ok(p) => p,
            Err(e) => {
                // NIST 800-53: AU-2 - Log security event
                if e.is_security_event() {
                    warn!("Security event during symlink (linkpath): {} - {}", linkpath, e);
                }
                return Ok(self.send_status_error(request_id, &e)?);
            }
        };

        debug!("Symlink request: {:?} -> {}", resolved_linkpath, targetpath);

        // NIST 800-53: AC-3 - Security validation
        // Check if symlink already exists
        if resolved_linkpath.exists() {
            warn!("Symlink creation failed: path already exists: {:?}", resolved_linkpath);
            return Ok(self.send_status_error(
                request_id,
                &Error::Other(format!("Path already exists: {}", linkpath)),
            )?);
        }

        // NIST 800-53: AC-3 - Security check on target
        // The target doesn't need to exist for symlink creation, but we should validate
        // that if it's an absolute path, it's within our root directory
        let target_path = PathBuf::from(&targetpath);
        if target_path.is_absolute() {
            // If target is absolute, it should be within root directory
            if !target_path.starts_with(&self.config.root_dir) {
                warn!(
                    "Symlink target points outside root directory: {} -> {}",
                    linkpath, targetpath
                );
                return Ok(self.send_status_error(
                    request_id,
                    &Error::PermissionDenied("Symlink target outside root directory".into()),
                )?);
            }
        }

        // NIST 800-53: AC-12 - Timeout protection for symlink creation
        use tokio::fs::symlink;
        let symlink_result = timeout(
            FILE_OP_TIMEOUT,
            symlink(&targetpath, &resolved_linkpath)
        ).await;

        match symlink_result {
            Ok(result) => match result {
                Ok(_) => {
                    info!("Created symlink: {:?} -> {}", resolved_linkpath, targetpath);
                    self.send_status(request_id, StatusCode::Ok, "Success")
                }
                Err(e) => {
                    debug!("Failed to create symlink {:?} -> {}: {}", resolved_linkpath, targetpath, e);
                    let error = if e.kind() == std::io::ErrorKind::PermissionDenied {
                        Error::PermissionDenied(format!("Cannot create symlink: {}", linkpath))
                    } else if e.kind() == std::io::ErrorKind::AlreadyExists {
                        Error::Other(format!("Symlink already exists: {}", linkpath))
                    } else {
                        Error::Io(e)
                    };
                    Ok(self.send_status_error(request_id, &error)?)
                }
            },
            Err(_) => {
                error!("Symlink operation timed out after {} seconds", FILE_OP_TIMEOUT.as_secs());
                Ok(self.send_status_error(
                    request_id,
                    &Error::timeout("Symlink operation timed out"),
                )?)
            }
        }
    }

    /// Create symbolic link (non-Unix fallback)
    #[cfg(not(unix))]
    async fn handle_symlink(&mut self, buf: &mut &[u8]) -> Result<Vec<u8>> {
        let request_id = self.read_u32(buf)?;
        let _linkpath = codec::get_string(buf)?;
        let _targetpath = codec::get_string(buf)?;

        warn!("SYMLINK not supported on this platform");
        Ok(self.send_status_error(
            request_id,
            &Error::NotSupported("SYMLINK not supported on this platform".into()),
        )?)
    }

    // Helper methods

    /// Resolve and validate path
    ///
    /// NIST 800-53: SI-10 (Input Validation), AC-3 (Access Enforcement)
    /// STIG: V-222396, V-222596
    /// Implementation: Prevents path traversal attacks and validates input
    fn resolve_path(&self, path: &str) -> Result<PathBuf> {
        // NIST 800-53: SI-10 - Validate input
        if path.is_empty() {
            return Err(Error::InvalidPath("Empty path".to_string()));
        }

        // NIST 800-53: SI-10 - Check for null bytes (security)
        if path.contains('\0') {
            warn!("Path contains null bytes: {:?}", path);
            return Err(Error::InvalidPath(
                "Path contains invalid characters".to_string(),
            ));
        }

        let path = if path.starts_with('/') {
            &path[1..]
        } else {
            path
        };

        let resolved = self.config.root_dir.join(path);

        // NIST 800-53: AC-3 - Ensure the path is within root_dir (prevent path traversal)
        // STIG: V-222396, V-222596
        if !resolved.starts_with(&self.config.root_dir) {
            warn!("Path traversal attempt detected: {}", path);
            return Err(Error::InvalidPath("Invalid path".to_string()));
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
        Ok(FileHandle::File(file, path))
    }

    /// Apply file attributes (permissions, timestamps, ownership)
    ///
    /// NIST 800-53: AC-3 (Access Enforcement)
    /// Implementation: Applies requested attribute changes with proper error handling
    async fn apply_file_attrs(&self, path: &PathBuf, attrs: &FileAttrs) -> Result<()> {
        // Apply permissions if specified
        #[cfg(unix)]
        if let Some(permissions) = attrs.permissions {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(permissions);
            timeout(FILE_OP_TIMEOUT, fs::set_permissions(path, perms))
                .await
                .map_err(|_| Error::timeout("Set permissions operation timed out"))?
                .map_err(|e| {
                    warn!("Failed to set permissions on {:?}: {}", path, e);
                    Error::PermissionDenied(format!("Cannot set permissions: {}", e))
                })?;
            info!("Set permissions {:o} on {:?}", permissions, path);
        }

        // Apply ownership if specified (requires appropriate privileges)
        #[cfg(unix)]
        if attrs.uid.is_some() || attrs.gid.is_some() {
            use std::os::unix::prelude::*;
            use tokio::fs::metadata;

            let meta = metadata(path).await?;
            let current_uid = meta.uid();
            let current_gid = meta.gid();

            let new_uid = attrs.uid.unwrap_or(current_uid);
            let new_gid = attrs.gid.unwrap_or(current_gid);

            // Note: chown requires root privileges in most cases
            // We attempt it but don't fail if it doesn't work
            #[cfg(target_os = "linux")]
            {
                use std::ffi::CString;
                use std::os::unix::ffi::OsStrExt;

                let path_c = CString::new(path.as_os_str().as_bytes())
                    .map_err(|_| Error::InvalidPath("Path contains null byte".into()))?;

                unsafe {
                    if libc::chown(path_c.as_ptr(), new_uid, new_gid) != 0 {
                        let err = std::io::Error::last_os_error();
                        warn!("Failed to set ownership on {:?}: {}", path, err);
                        // Don't fail - just log the warning
                        // This is expected when not running as root
                    } else {
                        info!("Set ownership uid={}, gid={} on {:?}", new_uid, new_gid, path);
                    }
                }
            }
        }

        // Apply timestamps if specified
        if attrs.atime.is_some() || attrs.mtime.is_some() {
            // Note: Setting atime/mtime requires platform-specific code
            // For now, we'll use a simplified approach with filetime crate if available
            // or log that it's not supported
            debug!("Timestamp modification requested but not fully implemented");
            // TODO: Implement timestamp modification using filetime crate or platform-specific APIs
        }

        Ok(())
    }

    fn allocate_handle(&mut self, handle: FileHandle) -> Vec<u8> {
        let id = self.next_handle_id;
        self.next_handle_id += 1;

        let handle_id = id.to_be_bytes().to_vec();
        self.handles.insert(handle_id.clone(), handle);
        handle_id
    }

    /// Send STATUS response with explicit code and message
    fn send_status(&self, request_id: u32, code: StatusCode, msg: &str) -> Result<Vec<u8>> {
        let mut response = BytesMut::new();
        response.put_u8(MessageType::Status as u8);
        response.put_u32(request_id);
        response.put_u32(code.into());
        codec::put_string(&mut response, msg);
        codec::put_string(&mut response, "en"); // language tag

        Ok(response.to_vec())
    }

    /// Send STATUS response from Error
    ///
    /// NIST 800-53: SI-11 (Error Handling)
    /// STIG: V-222566
    /// Implementation: Uses sanitized error messages and proper status codes
    fn send_status_error(&self, request_id: u32, error: &Error) -> Result<Vec<u8>> {
        let code = error.to_status_code();
        let msg = error.sanitized_message();

        let mut response = BytesMut::new();
        response.put_u8(MessageType::Status as u8);
        response.put_u32(request_id);
        response.put_u32(code);
        codec::put_string(&mut response, &msg);
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

/// File handle types
///
/// NIST 800-53: SI-11 (Error Handling)
/// Implementation: Proper resource cleanup via Drop trait
enum FileHandle {
    File(fs::File, PathBuf), // File and its path for fsetstat support
    Dir(DirHandle),
}

impl Drop for FileHandle {
    /// NIST 800-53: SI-11 - Ensure resources are cleaned up
    fn drop(&mut self) {
        match self {
            FileHandle::File(_, path) => {
                debug!("Closing file handle for {:?}", path);
            }
            FileHandle::Dir(_) => {
                debug!("Closing directory handle");
            }
        }
    }
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
