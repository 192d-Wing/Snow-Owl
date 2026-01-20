//! Error types for SFTP operations
//!
//! NIST 800-53: SI-11 (Error Handling), AU-3 (Content of Audit Records)
//! STIG: V-222566 - The application must generate error messages that provide information
//! necessary for corrective actions without revealing information that could be exploited
//! Implementation: Secure error messages with appropriate detail for troubleshooting

use thiserror::Error;

/// Result type alias for SFTP operations
pub type Result<T> = std::result::Result<T, Error>;

/// SFTP error types
///
/// NIST 800-53: SI-11 (Error Handling)
/// STIG: V-222566
/// Implementation: Error types that provide context without exposing sensitive information
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error
    ///
    /// NIST 800-53: SI-11
    /// Implementation: Wraps standard I/O errors with context
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// SSH protocol error
    ///
    /// NIST 800-53: SI-11, SC-8 (Transmission Confidentiality)
    /// Implementation: SSH-level errors (connection, encryption, etc.)
    #[error("SSH error: {0}")]
    Ssh(String),

    /// SFTP protocol error
    ///
    /// NIST 800-53: SI-11
    /// Implementation: SFTP protocol violations or invalid messages
    #[error("SFTP protocol error: {0}")]
    Protocol(String),

    /// Authentication failed
    ///
    /// NIST 800-53: IA-2 (Identification and Authentication), SI-11
    /// STIG: V-222566
    /// Implementation: Authentication errors without revealing why (security)
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// File not found
    ///
    /// NIST 800-53: SI-11
    /// Implementation: File or directory does not exist
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Permission denied
    ///
    /// NIST 800-53: AC-3 (Access Enforcement), SI-11
    /// STIG: V-222596, V-222566
    /// Implementation: Access control violation without exposing system details
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Invalid path
    ///
    /// NIST 800-53: SI-10 (Input Validation), SI-11
    /// STIG: V-222396, V-222566
    /// Implementation: Path validation failure (traversal, invalid characters, etc.)
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Configuration error
    ///
    /// NIST 800-53: SI-11
    /// Implementation: Invalid or missing configuration
    #[error("Configuration error: {0}")]
    Config(String),

    /// Connection error
    ///
    /// NIST 800-53: SC-8 (Transmission Confidentiality), SI-11
    /// Implementation: Network connection failures
    #[error("Connection error: {0}")]
    Connection(String),

    /// Timeout error
    ///
    /// NIST 800-53: AC-11 (Session Lock), AC-12 (Session Termination), SI-11
    /// STIG: V-222601
    /// Implementation: Operation exceeded time limit
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// File handle error
    ///
    /// NIST 800-53: SI-11
    /// Implementation: Invalid or closed file handle
    #[error("Invalid file handle: {0}")]
    InvalidHandle(String),

    /// Resource exhaustion
    ///
    /// NIST 800-53: SI-11
    /// Implementation: System resources depleted (handles, memory, etc.)
    #[error("Resource exhaustion: {0}")]
    ResourceExhaustion(String),

    /// Operation not supported
    ///
    /// NIST 800-53: SI-11
    /// Implementation: Unsupported SFTP operation or feature
    #[error("Operation not supported: {0}")]
    NotSupported(String),

    /// Channel closed
    ///
    /// NIST 800-53: SC-8, SI-11
    /// Implementation: SSH channel unexpectedly closed
    #[error("Channel closed: {0}")]
    ChannelClosed(String),

    /// Generic error
    ///
    /// NIST 800-53: SI-11
    /// Implementation: Catch-all for uncategorized errors
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Check if error is recoverable
    ///
    /// # Returns
    ///
    /// `true` if the operation can be retried, `false` if it's a permanent failure
    ///
    /// # NIST 800-53: SI-11 (Error Handling)
    /// # Implementation: Categorizes errors for recovery logic
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Error::Timeout(_) | Error::Connection(_) | Error::ChannelClosed(_)
        )
    }

    /// Check if error is due to client input
    ///
    /// # Returns
    ///
    /// `true` if error was caused by invalid client input
    ///
    /// # NIST 800-53: SI-10 (Input Validation), SI-11
    /// # STIG: V-222396, V-222566
    /// # Implementation: Identifies input validation failures
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            Error::InvalidPath(_)
                | Error::FileNotFound(_)
                | Error::PermissionDenied(_)
                | Error::InvalidHandle(_)
                | Error::NotSupported(_)
                | Error::Protocol(_)
        )
    }

    /// Check if error is security-related
    ///
    /// # Returns
    ///
    /// `true` if error has security implications
    ///
    /// # NIST 800-53: AU-2 (Audit Events), SI-11
    /// # STIG: V-222566
    /// # Implementation: Identifies errors that should be audited
    pub fn is_security_event(&self) -> bool {
        matches!(
            self,
            Error::Authentication(_) | Error::PermissionDenied(_) | Error::InvalidPath(_)
        )
    }

    /// Get error code for SFTP STATUS message
    ///
    /// # Returns
    ///
    /// SFTP status code corresponding to this error
    ///
    /// # NIST 800-53: SI-11
    /// # Implementation: Maps errors to RFC-compliant status codes
    pub fn to_status_code(&self) -> u32 {
        use crate::protocol::StatusCode;

        match self {
            Error::Io(_) => StatusCode::Failure as u32,
            Error::FileNotFound(_) => StatusCode::NoSuchFile as u32,
            Error::PermissionDenied(_) => StatusCode::PermissionDenied as u32,
            Error::InvalidPath(_) => StatusCode::BadMessage as u32,
            Error::InvalidHandle(_) => StatusCode::BadMessage as u32,
            Error::NotSupported(_) => StatusCode::OpUnsupported as u32,
            Error::Timeout(_) => StatusCode::Failure as u32,
            Error::Connection(_) | Error::ChannelClosed(_) => StatusCode::ConnectionLost as u32,
            _ => StatusCode::Failure as u32,
        }
    }

    /// Get sanitized error message for client
    ///
    /// # Returns
    ///
    /// Error message safe to send to client (no sensitive info)
    ///
    /// # NIST 800-53: SI-11 (Error Handling)
    /// # STIG: V-222566
    /// # Implementation: Removes sensitive information from error messages
    pub fn sanitized_message(&self) -> String {
        match self {
            Error::Authentication(_) => {
                // Don't reveal why authentication failed
                "Authentication failed".to_string()
            }
            Error::PermissionDenied(_) => {
                // Don't reveal path or reason
                "Permission denied".to_string()
            }
            Error::InvalidPath(_) => "Invalid path".to_string(),
            Error::Config(_) => {
                // Don't reveal configuration details
                "Server configuration error".to_string()
            }
            // For other errors, use the display message (already safe)
            _ => self.to_string(),
        }
    }
}

impl From<russh::Error> for Error {
    fn from(err: russh::Error) -> Self {
        Error::Ssh(err.to_string())
    }
}

impl From<russh_keys::Error> for Error {
    fn from(err: russh_keys::Error) -> Self {
        Error::Ssh(err.to_string())
    }
}

// Additional error constructors and helpers
impl Error {
    /// Create timeout error with context
    pub fn timeout(context: impl Into<String>) -> Self {
        Error::Timeout(context.into())
    }

    /// Create channel closed error
    pub fn channel_closed(context: impl Into<String>) -> Self {
        Error::ChannelClosed(context.into())
    }

    /// Create invalid handle error
    pub fn invalid_handle(context: impl Into<String>) -> Self {
        Error::InvalidHandle(context.into())
    }

    /// Create resource exhaustion error
    pub fn resource_exhaustion(context: impl Into<String>) -> Self {
        Error::ResourceExhaustion(context.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_recoverable() {
        assert!(Error::Timeout("test".into()).is_recoverable());
        assert!(Error::Connection("test".into()).is_recoverable());
        assert!(Error::ChannelClosed("test".into()).is_recoverable());
        assert!(!Error::FileNotFound("test".into()).is_recoverable());
        assert!(!Error::PermissionDenied("test".into()).is_recoverable());
    }

    #[test]
    fn test_is_client_error() {
        assert!(Error::InvalidPath("test".into()).is_client_error());
        assert!(Error::FileNotFound("test".into()).is_client_error());
        assert!(Error::PermissionDenied("test".into()).is_client_error());
        assert!(!Error::Connection("test".into()).is_client_error());
        assert!(!Error::Timeout("test".into()).is_client_error());
    }

    #[test]
    fn test_is_security_event() {
        assert!(Error::Authentication("test".into()).is_security_event());
        assert!(Error::PermissionDenied("test".into()).is_security_event());
        assert!(Error::InvalidPath("test".into()).is_security_event());
        assert!(!Error::FileNotFound("test".into()).is_security_event());
        assert!(!Error::Io(std::io::Error::from(std::io::ErrorKind::Other)).is_security_event());
    }

    #[test]
    fn test_sanitized_message() {
        let auth_err = Error::Authentication("Invalid key format".into());
        assert_eq!(auth_err.sanitized_message(), "Authentication failed");

        let perm_err = Error::PermissionDenied("/etc/shadow".into());
        assert_eq!(perm_err.sanitized_message(), "Permission denied");

        let config_err = Error::Config("Missing host_key at /secure/path".into());
        assert_eq!(
            config_err.sanitized_message(),
            "Server configuration error"
        );
    }

    #[test]
    fn test_to_status_code() {
        use crate::protocol::StatusCode;

        assert_eq!(
            Error::FileNotFound("test".into()).to_status_code(),
            StatusCode::NoSuchFile as u32
        );
        assert_eq!(
            Error::PermissionDenied("test".into()).to_status_code(),
            StatusCode::PermissionDenied as u32
        );
        assert_eq!(
            Error::NotSupported("test".into()).to_status_code(),
            StatusCode::OpUnsupported as u32
        );
    }
}
