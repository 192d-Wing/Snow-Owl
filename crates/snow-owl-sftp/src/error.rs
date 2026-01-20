//! Error types for SFTP operations

use thiserror::Error;

/// Result type alias for SFTP operations
pub type Result<T> = std::result::Result<T, Error>;

/// SFTP error types
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// SSH protocol error
    #[error("SSH error: {0}")]
    Ssh(String),

    /// SFTP protocol error
    #[error("SFTP protocol error: {0}")]
    Protocol(String),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Invalid path
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Timeout error
    #[error("Operation timed out")]
    Timeout,

    /// Generic error
    #[error("{0}")]
    Other(String),
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
