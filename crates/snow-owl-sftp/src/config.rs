//! Configuration for SFTP server and client

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// SFTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server bind address
    #[serde(default = "default_bind_address")]
    pub bind_address: String,

    /// Server port (default: 2222 for non-privileged, 22 for SSH standard)
    #[serde(default = "default_port")]
    pub port: u16,

    /// Root directory for SFTP file operations
    #[serde(default = "default_root_dir")]
    pub root_dir: PathBuf,

    /// SSH host key path
    #[serde(default = "default_host_key_path")]
    pub host_key_path: PathBuf,

    /// Authorized keys file path
    #[serde(default = "default_authorized_keys_path")]
    pub authorized_keys_path: PathBuf,

    /// Maximum concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Enable verbose logging
    #[serde(default)]
    pub verbose: bool,

    /// Maximum packet size (RFC 4254 recommends 32768 bytes minimum)
    #[serde(default = "default_max_packet_size")]
    pub max_packet_size: u32,

    /// Window size for flow control
    #[serde(default = "default_window_size")]
    pub window_size: u32,

    /// Maximum authentication attempts per IP (NIST 800-53: AC-7)
    #[serde(default = "default_max_auth_attempts")]
    pub max_auth_attempts: u32,

    /// Rate limit window in seconds (NIST 800-53: AC-7)
    #[serde(default = "default_rate_limit_window")]
    pub rate_limit_window_secs: u64,

    /// Lockout duration in seconds after max attempts (NIST 800-53: AC-7)
    #[serde(default = "default_lockout_duration")]
    pub lockout_duration_secs: u64,

    /// Maximum connections per user (AC-12: Session Termination)
    #[serde(default = "default_max_connections_per_user")]
    pub max_connections_per_user: usize,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
}

/// Logging configuration
///
/// NIST 800-53: AU-2 (Audit Events), AU-9 (Protection of Audit Information), AU-12 (Audit Generation)
/// STIG: V-222648 (Audit Records)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Log format (text or json)
    pub format: LogFormat,
    /// Optional log file path (logs to stderr if not specified)
    pub file: Option<PathBuf>,
    /// Enable structured audit logging for SIEM integration
    /// When enabled, all security-relevant events are logged as structured JSON
    pub audit_enabled: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Json,
            file: Some(PathBuf::from("/var/log/snow-owl/sftp-audit.json")),
            audit_enabled: true,
        }
    }
}

/// Log format options
///
/// NIST 800-53: AU-9 (Protection of Audit Information)
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// Plain text logging for human readability
    Text,
    /// JSON structured logging for SIEM integration
    /// All log entries are formatted as JSON for easy parsing by log aggregators
    Json,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_address: default_bind_address(),
            port: default_port(),
            root_dir: default_root_dir(),
            host_key_path: default_host_key_path(),
            authorized_keys_path: default_authorized_keys_path(),
            max_connections: default_max_connections(),
            timeout: default_timeout(),
            verbose: false,
            max_packet_size: default_max_packet_size(),
            window_size: default_window_size(),
            max_auth_attempts: default_max_auth_attempts(),
            rate_limit_window_secs: default_rate_limit_window(),
            lockout_duration_secs: default_lockout_duration(),
            max_connections_per_user: default_max_connections_per_user(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file(path: &str) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::Error::Config(format!("Failed to read config file: {}", e)))?;

        toml::from_str(&content)
            .map_err(|e| crate::Error::Config(format!("Failed to parse config: {}", e)))
    }

    /// Validate configuration
    pub fn validate(&self) -> crate::Result<()> {
        if !self.root_dir.exists() {
            return Err(crate::Error::Config(format!(
                "Root directory does not exist: {:?}",
                self.root_dir
            )));
        }

        if !self.root_dir.is_dir() {
            return Err(crate::Error::Config(format!(
                "Root path is not a directory: {:?}",
                self.root_dir
            )));
        }

        if self.max_packet_size < 32768 {
            return Err(crate::Error::Config(
                "max_packet_size must be at least 32768 bytes (RFC 4254)".to_string()
            ));
        }

        Ok(())
    }
}

fn default_bind_address() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    2222 // Non-privileged port for testing
}

fn default_root_dir() -> PathBuf {
    PathBuf::from("/tmp/sftp")
}

fn default_host_key_path() -> PathBuf {
    PathBuf::from("/etc/ssh/ssh_host_rsa_key")
}

fn default_authorized_keys_path() -> PathBuf {
    PathBuf::from("~/.ssh/authorized_keys")
}

fn default_max_connections() -> usize {
    100
}

fn default_timeout() -> u64 {
    300 // 5 minutes
}

fn default_max_packet_size() -> u32 {
    32768 // RFC 4254 minimum
}

fn default_window_size() -> u32 {
    2097152 // 2MB
}

// NIST 800-53: AC-7 (Unsuccessful Logon Attempts)
// Default: 5 attempts before lockout
fn default_max_auth_attempts() -> u32 {
    5
}

// NIST 800-53: AC-7 (Unsuccessful Logon Attempts)
// Default: 5 minute window
fn default_rate_limit_window() -> u64 {
    300 // 5 minutes
}

// NIST 800-53: AC-7 (Unsuccessful Logon Attempts)
// Default: 15 minute lockout
fn default_lockout_duration() -> u64 {
    900 // 15 minutes
}

// NIST 800-53: AC-12 (Session Termination)
// Default: 10 connections per user
fn default_max_connections_per_user() -> usize {
    10
}
