//! Configuration for SFTP server and client

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::net::IpAddr;

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

    /// Per-user configurations (NIST 800-53: AC-3, AC-6)
    #[serde(default)]
    pub users: HashMap<String, UserConfig>,

    /// Global bandwidth limit in bytes per second (0 = unlimited)
    #[serde(default)]
    pub global_bandwidth_limit: u64,

    /// IP whitelist - if not empty, only these IPs are allowed
    #[serde(default)]
    pub ip_whitelist: Vec<IpAddr>,

    /// IP blacklist - these IPs are always denied
    #[serde(default)]
    pub ip_blacklist: Vec<IpAddr>,

    /// Configuration file path for hot reload
    #[serde(skip)]
    pub config_file_path: Option<PathBuf>,
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

/// Per-user configuration
///
/// NIST 800-53: AC-3 (Access Enforcement), AC-6 (Least Privilege)
/// STIG: V-222567 (User Access Control)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UserConfig {
    /// User's home directory (chroot jail)
    /// If set, user is restricted to this directory and subdirectories
    pub home_dir: Option<PathBuf>,

    /// User-specific bandwidth limit in bytes per second (0 = use global limit)
    pub bandwidth_limit: u64,

    /// Disk quota in bytes (0 = unlimited)
    pub disk_quota: u64,

    /// Maximum file size in bytes (0 = unlimited)
    pub max_file_size: u64,

    /// Maximum number of concurrent connections for this user
    pub max_connections: Option<usize>,

    /// Time-based access restrictions
    pub access_schedule: Option<AccessSchedule>,

    /// Read-only mode (user can only download, not upload or modify)
    pub read_only: bool,

    /// Allowed operations - if specified, only these operations are permitted
    pub allowed_operations: Option<Vec<String>>,

    /// Denied operations - these operations are explicitly forbidden
    pub denied_operations: Vec<String>,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            home_dir: None,
            bandwidth_limit: 0,
            disk_quota: 0,
            max_file_size: 0,
            max_connections: None,
            access_schedule: None,
            read_only: false,
            allowed_operations: None,
            denied_operations: Vec::new(),
        }
    }
}

/// Access schedule configuration for time-based restrictions
///
/// NIST 800-53: AC-2 (Account Management)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessSchedule {
    /// Days of week when access is allowed (0 = Sunday, 6 = Saturday)
    /// Empty vec = all days allowed
    pub allowed_days: Vec<u8>,

    /// Start hour (0-23) when access is allowed
    pub start_hour: u8,

    /// End hour (0-23) when access is allowed
    pub end_hour: u8,

    /// Timezone for schedule (e.g., "America/New_York", "UTC")
    pub timezone: String,
}

impl Default for AccessSchedule {
    fn default() -> Self {
        Self {
            allowed_days: vec![1, 2, 3, 4, 5], // Monday-Friday
            start_hour: 9,
            end_hour: 17,
            timezone: "UTC".to_string(),
        }
    }
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
            users: HashMap::new(),
            global_bandwidth_limit: 0,
            ip_whitelist: Vec::new(),
            ip_blacklist: Vec::new(),
            config_file_path: None,
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file(path: &str) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::Error::Config(format!("Failed to read config file: {}", e)))?;

        let mut config: Self = toml::from_str(&content)
            .map_err(|e| crate::Error::Config(format!("Failed to parse config: {}", e)))?;

        // Store config file path for hot reload
        config.config_file_path = Some(PathBuf::from(path));

        Ok(config)
    }

    /// Reload configuration from the original file
    ///
    /// NIST 800-53: CM-3 (Configuration Change Control)
    pub fn reload(&mut self) -> crate::Result<()> {
        if let Some(ref path) = self.config_file_path {
            let new_config = Self::from_file(
                path.to_str()
                    .ok_or_else(|| crate::Error::Config("Invalid config path".to_string()))?
            )?;

            // Preserve connection-specific state but update configuration
            *self = new_config;

            Ok(())
        } else {
            Err(crate::Error::Config(
                "No config file path available for reload".to_string()
            ))
        }
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

        // Validate per-user configurations
        for (username, user_config) in &self.users {
            if let Some(ref home_dir) = user_config.home_dir {
                if !home_dir.exists() {
                    return Err(crate::Error::Config(format!(
                        "User '{}' home directory does not exist: {:?}",
                        username, home_dir
                    )));
                }
                if !home_dir.is_dir() {
                    return Err(crate::Error::Config(format!(
                        "User '{}' home path is not a directory: {:?}",
                        username, home_dir
                    )));
                }
            }

            if let Some(ref schedule) = user_config.access_schedule {
                if schedule.start_hour > 23 || schedule.end_hour > 23 {
                    return Err(crate::Error::Config(format!(
                        "User '{}' has invalid access schedule hours (must be 0-23)",
                        username
                    )));
                }
                for &day in &schedule.allowed_days {
                    if day > 6 {
                        return Err(crate::Error::Config(format!(
                            "User '{}' has invalid day in access schedule (must be 0-6)",
                            username
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// Get user-specific configuration
    pub fn get_user_config(&self, username: &str) -> Option<&UserConfig> {
        self.users.get(username)
    }

    /// Check if an IP address is allowed to connect
    ///
    /// NIST 800-53: AC-3 (Access Enforcement)
    /// STIG: V-222567 (Access Control)
    pub fn is_ip_allowed(&self, ip: &IpAddr) -> bool {
        // First check blacklist - always deny
        if self.ip_blacklist.contains(ip) {
            return false;
        }

        // If whitelist is empty, allow all (except blacklisted)
        if self.ip_whitelist.is_empty() {
            return true;
        }

        // Otherwise, must be in whitelist
        self.ip_whitelist.contains(ip)
    }

    /// Check if a user can access at the current time
    ///
    /// NIST 800-53: AC-2 (Account Management)
    pub fn is_access_time_allowed(&self, username: &str) -> bool {
        if let Some(user_config) = self.get_user_config(username) {
            if let Some(ref schedule) = user_config.access_schedule {
                use chrono::{Datelike, Timelike, Utc};

                let now = Utc::now();
                let day_of_week = now.weekday().num_days_from_sunday() as u8;
                let hour = now.hour() as u8;

                // Check day of week
                if !schedule.allowed_days.is_empty() && !schedule.allowed_days.contains(&day_of_week) {
                    return false;
                }

                // Check hour range
                if hour < schedule.start_hour || hour >= schedule.end_hour {
                    return false;
                }
            }
        }

        true
    }

    /// Check if a user can perform a specific operation
    ///
    /// NIST 800-53: AC-3 (Access Enforcement)
    pub fn is_operation_allowed(&self, username: &str, operation: &str) -> bool {
        if let Some(user_config) = self.get_user_config(username) {
            // Check denied operations first
            if user_config.denied_operations.contains(&operation.to_string()) {
                return false;
            }

            // If allowed_operations is set, operation must be in the list
            if let Some(ref allowed) = user_config.allowed_operations {
                return allowed.contains(&operation.to_string());
            }

            // Check read-only mode
            if user_config.read_only {
                let read_only_ops = vec!["read", "stat", "lstat", "opendir", "readdir", "readlink"];
                return read_only_ops.contains(&operation);
            }
        }

        true
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
