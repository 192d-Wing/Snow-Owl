//! Audit trail and session tracking
//!
//! NIST 800-53: AU-2 (Audit Events), AU-3 (Content of Audit Records), AU-12 (Audit Generation)
//! STIG: V-222648 (Audit Records), V-222566 (Monitoring)
//! Implementation: Comprehensive audit logging for security and compliance

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::PathBuf;
use tracing::{info, warn};

/// Audit event types
///
/// NIST 800-53: AU-2 (Audit Events)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum AuditEvent {
    /// Connection established
    ConnectionEstablished {
        /// Client IP address
        client_ip: Option<IpAddr>,
        /// Event timestamp
        timestamp: DateTime<Utc>,
    },
    /// Connection closed
    ConnectionClosed {
        /// Client IP address
        client_ip: Option<IpAddr>,
        /// Authenticated username
        username: Option<String>,
        /// Event timestamp
        timestamp: DateTime<Utc>,
        /// Session duration in seconds
        duration_secs: i64,
    },
    /// Authentication attempt
    AuthAttempt {
        /// Client IP address
        client_ip: Option<IpAddr>,
        /// Username attempted
        username: String,
        /// Event timestamp
        timestamp: DateTime<Utc>,
        /// Whether authentication succeeded
        success: bool,
        /// Failure reason if applicable
        reason: Option<String>,
    },
    /// File operation
    FileOperation {
        /// Client IP address
        client_ip: Option<IpAddr>,
        /// Authenticated username
        username: Option<String>,
        /// Operation type (read, write, etc.)
        operation: String,
        /// File path
        path: String,
        /// Event timestamp
        timestamp: DateTime<Utc>,
        /// Whether operation succeeded
        success: bool,
        /// Bytes transferred if applicable
        bytes_transferred: Option<u64>,
        /// Error message if failed
        error: Option<String>,
    },
    /// Directory operation
    DirectoryOperation {
        /// Client IP address
        client_ip: Option<IpAddr>,
        /// Authenticated username
        username: Option<String>,
        /// Operation type (mkdir, rmdir, etc.)
        operation: String,
        /// Directory path
        path: String,
        /// Event timestamp
        timestamp: DateTime<Utc>,
        /// Whether operation succeeded
        success: bool,
        /// Error message if failed
        error: Option<String>,
    },
    /// Security event
    SecurityEvent {
        /// Client IP address
        client_ip: Option<IpAddr>,
        /// Authenticated username
        username: Option<String>,
        /// Security event type
        event: String,
        /// Event details
        details: String,
        /// Event timestamp
        timestamp: DateTime<Utc>,
    },
    /// Rate limit triggered
    RateLimitTriggered {
        /// Client IP address
        client_ip: Option<IpAddr>,
        /// Event timestamp
        timestamp: DateTime<Utc>,
        /// Lockout duration in seconds
        duration_secs: u64,
    },
    /// Connection limit reached
    ConnectionLimitReached {
        /// Username that hit the limit
        username: String,
        /// Current connection count
        current_connections: usize,
        /// Maximum allowed connections
        max_connections: usize,
        /// Event timestamp
        timestamp: DateTime<Utc>,
    },
}

impl AuditEvent {
    /// Log the audit event
    ///
    /// NIST 800-53: AU-12 (Audit Generation)
    /// Implementation: Structured logging of audit events
    pub fn log(&self) {
        match self {
            AuditEvent::ConnectionEstablished { client_ip, .. } => {
                info!(
                    event = "connection_established",
                    client_ip = ?client_ip,
                    audit = ?self,
                    "New connection established"
                );
            }
            AuditEvent::ConnectionClosed {
                username,
                duration_secs,
                ..
            } => {
                info!(
                    event = "connection_closed",
                    username = ?username,
                    duration_secs,
                    audit = ?self,
                    "Connection closed"
                );
            }
            AuditEvent::AuthAttempt {
                username,
                success,
                reason,
                ..
            } => {
                if *success {
                    info!(
                        event = "auth_success",
                        username,
                        audit = ?self,
                        "Authentication successful"
                    );
                } else {
                    warn!(
                        event = "auth_failure",
                        username,
                        reason = ?reason,
                        audit = ?self,
                        "Authentication failed"
                    );
                }
            }
            AuditEvent::FileOperation {
                username,
                operation,
                path,
                success,
                bytes_transferred,
                error,
                ..
            } => {
                if *success {
                    info!(
                        event = "file_operation",
                        username = ?username,
                        operation,
                        path,
                        bytes = ?bytes_transferred,
                        audit = ?self,
                        "File operation completed"
                    );
                } else {
                    warn!(
                        event = "file_operation_failed",
                        username = ?username,
                        operation,
                        path,
                        error = ?error,
                        audit = ?self,
                        "File operation failed"
                    );
                }
            }
            AuditEvent::DirectoryOperation {
                username,
                operation,
                path,
                success,
                error,
                ..
            } => {
                if *success {
                    info!(
                        event = "directory_operation",
                        username = ?username,
                        operation,
                        path,
                        audit = ?self,
                        "Directory operation completed"
                    );
                } else {
                    warn!(
                        event = "directory_operation_failed",
                        username = ?username,
                        operation,
                        path,
                        error = ?error,
                        audit = ?self,
                        "Directory operation failed"
                    );
                }
            }
            AuditEvent::SecurityEvent {
                username,
                event,
                details,
                ..
            } => {
                warn!(
                    event = "security_event",
                    username = ?username,
                    security_event = event,
                    details,
                    audit = ?self,
                    "Security event detected"
                );
            }
            AuditEvent::RateLimitTriggered {
                client_ip,
                duration_secs,
                ..
            } => {
                warn!(
                    event = "rate_limit_triggered",
                    client_ip = ?client_ip,
                    duration_secs,
                    audit = ?self,
                    "Rate limit triggered"
                );
            }
            AuditEvent::ConnectionLimitReached {
                username,
                current_connections,
                max_connections,
                ..
            } => {
                warn!(
                    event = "connection_limit_reached",
                    username,
                    current_connections,
                    max_connections,
                    audit = ?self,
                    "Connection limit reached"
                );
            }
        }
    }

    /// Export as JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Export as pretty JSON
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Session information tracker
///
/// NIST 800-53: AU-3 (Content of Audit Records)
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// Unique session identifier
    pub session_id: String,
    /// Client IP address
    pub client_ip: Option<IpAddr>,
    /// Authenticated username
    pub username: Option<String>,
    /// Session start time
    pub start_time: DateTime<Utc>,
    /// Time of last activity
    pub last_activity: DateTime<Utc>,
}

impl SessionInfo {
    /// Create a new session info
    pub fn new(session_id: String, client_ip: Option<IpAddr>) -> Self {
        let now = Utc::now();
        Self {
            session_id,
            client_ip,
            username: None,
            start_time: now,
            last_activity: now,
        }
    }

    /// Update last activity time
    pub fn update_activity(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Set username after authentication
    pub fn set_username(&mut self, username: String) {
        self.username = Some(username);
    }

    /// Get session duration in seconds
    pub fn duration_secs(&self) -> i64 {
        Utc::now()
            .signed_duration_since(self.start_time)
            .num_seconds()
    }
}

/// Audit logger for file operations
///
/// NIST 800-53: AU-2 (Audit Events), AU-12 (Audit Generation)
/// Implementation: Helper functions for common audit events
pub struct AuditLogger;

impl AuditLogger {
    /// Log a file read
    pub fn log_file_read(
        client_ip: Option<IpAddr>,
        username: Option<String>,
        path: &PathBuf,
        bytes: u64,
        success: bool,
        error: Option<String>,
    ) {
        let event = AuditEvent::FileOperation {
            client_ip,
            username,
            operation: "READ".to_string(),
            path: path.display().to_string(),
            timestamp: Utc::now(),
            success,
            bytes_transferred: Some(bytes),
            error,
        };
        event.log();
    }

    /// Log a file write
    pub fn log_file_write(
        client_ip: Option<IpAddr>,
        username: Option<String>,
        path: &PathBuf,
        bytes: u64,
        success: bool,
        error: Option<String>,
    ) {
        let event = AuditEvent::FileOperation {
            client_ip,
            username,
            operation: "WRITE".to_string(),
            path: path.display().to_string(),
            timestamp: Utc::now(),
            success,
            bytes_transferred: Some(bytes),
            error,
        };
        event.log();
    }

    /// Log a file delete
    pub fn log_file_delete(
        client_ip: Option<IpAddr>,
        username: Option<String>,
        path: &PathBuf,
        success: bool,
        error: Option<String>,
    ) {
        let event = AuditEvent::FileOperation {
            client_ip,
            username,
            operation: "DELETE".to_string(),
            path: path.display().to_string(),
            timestamp: Utc::now(),
            success,
            bytes_transferred: None,
            error,
        };
        event.log();
    }

    /// Log a file rename
    pub fn log_file_rename(
        client_ip: Option<IpAddr>,
        username: Option<String>,
        old_path: &PathBuf,
        new_path: &PathBuf,
        success: bool,
        error: Option<String>,
    ) {
        let event = AuditEvent::FileOperation {
            client_ip,
            username,
            operation: "RENAME".to_string(),
            path: format!("{} -> {}", old_path.display(), new_path.display()),
            timestamp: Utc::now(),
            success,
            bytes_transferred: None,
            error,
        };
        event.log();
    }

    /// Log a security event
    pub fn log_security_event(
        client_ip: Option<IpAddr>,
        username: Option<String>,
        event: String,
        details: String,
    ) {
        let audit_event = AuditEvent::SecurityEvent {
            client_ip,
            username,
            event,
            details,
            timestamp: Utc::now(),
        };
        audit_event.log();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::AuthAttempt {
            client_ip: Some("127.0.0.1".parse::<IpAddr>().ok()),
            username: "testuser".to_string(),
            timestamp: Utc::now(),
            success: true,
            reason: None,
        };

        let json = event.to_json().expect("JSON serialization failed");
        assert!(json.contains("Auth Attempt"));
    }

    #[test]
    fn test_session_info() {
        let mut session = SessionInfo::new(
            "test-session".to_string(),
            Some("127.0.0.1".parse().ok()),
        );

        assert_eq!(session.session_id, "test-session");
        assert!(session.username.is_none());

        session.set_username("testuser".to_string());
        assert_eq!(session.username.as_deref(), Some("testuser"));

        session.update_activity();
        assert!(session.duration_secs() >= 0);
    }

    #[test]
    fn test_file_operation_audit() {
        let path = PathBuf::from("/test/file.txt");
        AuditLogger::log_file_read(
            Some("127.0.0.1".parse().ok()),
            Some("testuser".to_string()),
            &path,
            1024,
            true,
            None,
        );
        // Test passes if no panic
    }
}
