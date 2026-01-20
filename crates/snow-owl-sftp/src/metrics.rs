//! Metrics collection and monitoring
//!
//! NIST 800-53: AU-2 (Audit Events), AU-12 (Audit Generation), SI-4 (System Monitoring)
//! STIG: V-222566 (Monitoring), V-222648 (Audit Records)
//! Implementation: Comprehensive metrics tracking for server operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Server-wide metrics collection
///
/// NIST 800-53: SI-4 (System Monitoring)
/// Implementation: Thread-safe metrics using atomic operations
#[derive(Debug, Clone)]
pub struct Metrics {
    inner: Arc<MetricsInner>,
}

#[derive(Debug)]
struct MetricsInner {
    // Connection metrics
    total_connections: AtomicU64,
    active_connections: AtomicUsize,
    failed_connections: AtomicU64,
    rejected_connections: AtomicU64,

    // Authentication metrics
    auth_attempts: AtomicU64,
    auth_successes: AtomicU64,
    auth_failures: AtomicU64,
    rate_limited_attempts: AtomicU64,

    // File operation metrics
    file_opens: AtomicU64,
    file_reads: AtomicU64,
    file_writes: AtomicU64,
    file_closes: AtomicU64,
    file_removes: AtomicU64,
    file_renames: AtomicU64,

    // Directory operation metrics
    dir_opens: AtomicU64,
    dir_reads: AtomicU64,
    dir_creates: AtomicU64,
    dir_removes: AtomicU64,

    // Advanced operation metrics
    stat_operations: AtomicU64,
    setstat_operations: AtomicU64,
    symlink_operations: AtomicU64,
    readlink_operations: AtomicU64,

    // Data transfer metrics
    bytes_read: AtomicU64,
    bytes_written: AtomicU64,

    // Error metrics
    protocol_errors: AtomicU64,
    permission_denied: AtomicU64,
    file_not_found: AtomicU64,
    io_errors: AtomicU64,
    timeout_errors: AtomicU64,

    // Performance metrics
    total_operations: AtomicU64,

    // Server start time
    start_time: DateTime<Utc>,
}

/// Snapshot of current metrics
///
/// NIST 800-53: AU-2 (Audit Events)
/// Implementation: Serializable metrics snapshot for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    // Timestamp of snapshot
    pub timestamp: DateTime<Utc>,
    pub uptime_seconds: i64,

    // Connection metrics
    pub total_connections: u64,
    pub active_connections: usize,
    pub failed_connections: u64,
    pub rejected_connections: u64,

    // Authentication metrics
    pub auth_attempts: u64,
    pub auth_successes: u64,
    pub auth_failures: u64,
    pub rate_limited_attempts: u64,
    pub auth_success_rate: f64,

    // File operation metrics
    pub file_opens: u64,
    pub file_reads: u64,
    pub file_writes: u64,
    pub file_closes: u64,
    pub file_removes: u64,
    pub file_renames: u64,

    // Directory operation metrics
    pub dir_opens: u64,
    pub dir_reads: u64,
    pub dir_creates: u64,
    pub dir_removes: u64,

    // Advanced operation metrics
    pub stat_operations: u64,
    pub setstat_operations: u64,
    pub symlink_operations: u64,
    pub readlink_operations: u64,

    // Data transfer metrics
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub total_bytes: u64,

    // Error metrics
    pub protocol_errors: u64,
    pub permission_denied: u64,
    pub file_not_found: u64,
    pub io_errors: u64,
    pub timeout_errors: u64,
    pub total_errors: u64,

    // Performance metrics
    pub total_operations: u64,
    pub operations_per_second: f64,
}

/// Operation timing tracker
///
/// NIST 800-53: SI-4 (System Monitoring)
/// Implementation: Track operation duration for performance analysis
#[derive(Debug)]
pub struct OperationTimer {
    start: Instant,
    operation_name: &'static str,
}

impl Metrics {
    /// Create a new metrics instance
    ///
    /// NIST 800-53: AU-12 (Audit Generation)
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MetricsInner {
                total_connections: AtomicU64::new(0),
                active_connections: AtomicUsize::new(0),
                failed_connections: AtomicU64::new(0),
                rejected_connections: AtomicU64::new(0),
                auth_attempts: AtomicU64::new(0),
                auth_successes: AtomicU64::new(0),
                auth_failures: AtomicU64::new(0),
                rate_limited_attempts: AtomicU64::new(0),
                file_opens: AtomicU64::new(0),
                file_reads: AtomicU64::new(0),
                file_writes: AtomicU64::new(0),
                file_closes: AtomicU64::new(0),
                file_removes: AtomicU64::new(0),
                file_renames: AtomicU64::new(0),
                dir_opens: AtomicU64::new(0),
                dir_reads: AtomicU64::new(0),
                dir_creates: AtomicU64::new(0),
                dir_removes: AtomicU64::new(0),
                stat_operations: AtomicU64::new(0),
                setstat_operations: AtomicU64::new(0),
                symlink_operations: AtomicU64::new(0),
                readlink_operations: AtomicU64::new(0),
                bytes_read: AtomicU64::new(0),
                bytes_written: AtomicU64::new(0),
                protocol_errors: AtomicU64::new(0),
                permission_denied: AtomicU64::new(0),
                file_not_found: AtomicU64::new(0),
                io_errors: AtomicU64::new(0),
                timeout_errors: AtomicU64::new(0),
                total_operations: AtomicU64::new(0),
                start_time: Utc::now(),
            }),
        }
    }

    // Connection metrics

    /// Record a new connection
    pub fn record_connection(&self) {
        self.inner.total_connections.fetch_add(1, Ordering::Relaxed);
        self.inner.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a connection close
    pub fn record_connection_close(&self) {
        self.inner.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record a failed connection
    pub fn record_failed_connection(&self) {
        self.inner.failed_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a rejected connection
    pub fn record_rejected_connection(&self) {
        self.inner.rejected_connections.fetch_add(1, Ordering::Relaxed);
    }

    // Authentication metrics

    /// Record an authentication attempt
    pub fn record_auth_attempt(&self) {
        self.inner.auth_attempts.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a successful authentication
    pub fn record_auth_success(&self) {
        self.inner.auth_successes.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed authentication
    pub fn record_auth_failure(&self) {
        self.inner.auth_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a rate-limited authentication attempt
    pub fn record_rate_limited(&self) {
        self.inner.rate_limited_attempts.fetch_add(1, Ordering::Relaxed);
    }

    // File operation metrics

    /// Record a file open
    pub fn record_file_open(&self) {
        self.inner.file_opens.fetch_add(1, Ordering::Relaxed);
        self.inner.total_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a file read
    pub fn record_file_read(&self, bytes: u64) {
        self.inner.file_reads.fetch_add(1, Ordering::Relaxed);
        self.inner.bytes_read.fetch_add(bytes, Ordering::Relaxed);
        self.inner.total_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a file write
    pub fn record_file_write(&self, bytes: u64) {
        self.inner.file_writes.fetch_add(1, Ordering::Relaxed);
        self.inner.bytes_written.fetch_add(bytes, Ordering::Relaxed);
        self.inner.total_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a file close
    pub fn record_file_close(&self) {
        self.inner.file_closes.fetch_add(1, Ordering::Relaxed);
        self.inner.total_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a file remove
    pub fn record_file_remove(&self) {
        self.inner.file_removes.fetch_add(1, Ordering::Relaxed);
        self.inner.total_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a file rename
    pub fn record_file_rename(&self) {
        self.inner.file_renames.fetch_add(1, Ordering::Relaxed);
        self.inner.total_operations.fetch_add(1, Ordering::Relaxed);
    }

    // Directory operation metrics

    /// Record a directory open
    pub fn record_dir_open(&self) {
        self.inner.dir_opens.fetch_add(1, Ordering::Relaxed);
        self.inner.total_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a directory read
    pub fn record_dir_read(&self) {
        self.inner.dir_reads.fetch_add(1, Ordering::Relaxed);
        self.inner.total_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a directory create
    pub fn record_dir_create(&self) {
        self.inner.dir_creates.fetch_add(1, Ordering::Relaxed);
        self.inner.total_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a directory remove
    pub fn record_dir_remove(&self) {
        self.inner.dir_removes.fetch_add(1, Ordering::Relaxed);
        self.inner.total_operations.fetch_add(1, Ordering::Relaxed);
    }

    // Advanced operation metrics

    /// Record a stat operation
    pub fn record_stat(&self) {
        self.inner.stat_operations.fetch_add(1, Ordering::Relaxed);
        self.inner.total_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a setstat operation
    pub fn record_setstat(&self) {
        self.inner.setstat_operations.fetch_add(1, Ordering::Relaxed);
        self.inner.total_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a symlink operation
    pub fn record_symlink(&self) {
        self.inner.symlink_operations.fetch_add(1, Ordering::Relaxed);
        self.inner.total_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a readlink operation
    pub fn record_readlink(&self) {
        self.inner.readlink_operations.fetch_add(1, Ordering::Relaxed);
        self.inner.total_operations.fetch_add(1, Ordering::Relaxed);
    }

    // Error metrics

    /// Record a protocol error
    pub fn record_protocol_error(&self) {
        self.inner.protocol_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a permission denied error
    pub fn record_permission_denied(&self) {
        self.inner.permission_denied.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a file not found error
    pub fn record_file_not_found(&self) {
        self.inner.file_not_found.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an I/O error
    pub fn record_io_error(&self) {
        self.inner.io_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a timeout error
    pub fn record_timeout_error(&self) {
        self.inner.timeout_errors.fetch_add(1, Ordering::Relaxed);
    }

    // Snapshot and reporting

    /// Get a snapshot of current metrics
    ///
    /// NIST 800-53: AU-2 (Audit Events)
    /// Implementation: Create a consistent snapshot for reporting
    pub fn snapshot(&self) -> MetricsSnapshot {
        let now = Utc::now();
        let uptime = now.signed_duration_since(self.inner.start_time);

        let auth_attempts = self.inner.auth_attempts.load(Ordering::Relaxed);
        let auth_successes = self.inner.auth_successes.load(Ordering::Relaxed);
        let auth_success_rate = if auth_attempts > 0 {
            (auth_successes as f64 / auth_attempts as f64) * 100.0
        } else {
            0.0
        };

        let bytes_read = self.inner.bytes_read.load(Ordering::Relaxed);
        let bytes_written = self.inner.bytes_written.load(Ordering::Relaxed);

        let protocol_errors = self.inner.protocol_errors.load(Ordering::Relaxed);
        let permission_denied = self.inner.permission_denied.load(Ordering::Relaxed);
        let file_not_found = self.inner.file_not_found.load(Ordering::Relaxed);
        let io_errors = self.inner.io_errors.load(Ordering::Relaxed);
        let timeout_errors = self.inner.timeout_errors.load(Ordering::Relaxed);
        let total_errors = protocol_errors + permission_denied + file_not_found + io_errors + timeout_errors;

        let total_operations = self.inner.total_operations.load(Ordering::Relaxed);
        let operations_per_second = if uptime.num_seconds() > 0 {
            total_operations as f64 / uptime.num_seconds() as f64
        } else {
            0.0
        };

        MetricsSnapshot {
            timestamp: now,
            uptime_seconds: uptime.num_seconds(),
            total_connections: self.inner.total_connections.load(Ordering::Relaxed),
            active_connections: self.inner.active_connections.load(Ordering::Relaxed),
            failed_connections: self.inner.failed_connections.load(Ordering::Relaxed),
            rejected_connections: self.inner.rejected_connections.load(Ordering::Relaxed),
            auth_attempts,
            auth_successes,
            auth_failures: self.inner.auth_failures.load(Ordering::Relaxed),
            rate_limited_attempts: self.inner.rate_limited_attempts.load(Ordering::Relaxed),
            auth_success_rate,
            file_opens: self.inner.file_opens.load(Ordering::Relaxed),
            file_reads: self.inner.file_reads.load(Ordering::Relaxed),
            file_writes: self.inner.file_writes.load(Ordering::Relaxed),
            file_closes: self.inner.file_closes.load(Ordering::Relaxed),
            file_removes: self.inner.file_removes.load(Ordering::Relaxed),
            file_renames: self.inner.file_renames.load(Ordering::Relaxed),
            dir_opens: self.inner.dir_opens.load(Ordering::Relaxed),
            dir_reads: self.inner.dir_reads.load(Ordering::Relaxed),
            dir_creates: self.inner.dir_creates.load(Ordering::Relaxed),
            dir_removes: self.inner.dir_removes.load(Ordering::Relaxed),
            stat_operations: self.inner.stat_operations.load(Ordering::Relaxed),
            setstat_operations: self.inner.setstat_operations.load(Ordering::Relaxed),
            symlink_operations: self.inner.symlink_operations.load(Ordering::Relaxed),
            readlink_operations: self.inner.readlink_operations.load(Ordering::Relaxed),
            bytes_read,
            bytes_written,
            total_bytes: bytes_read + bytes_written,
            protocol_errors,
            permission_denied,
            file_not_found,
            io_errors,
            timeout_errors,
            total_errors,
            total_operations,
            operations_per_second,
        }
    }

    /// Export metrics as JSON
    ///
    /// NIST 800-53: AU-2 (Audit Events)
    /// Implementation: JSON export for monitoring systems
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let snapshot = self.snapshot();
        serde_json::to_string_pretty(&snapshot)
    }

    /// Export metrics as compact JSON (single line)
    pub fn to_json_compact(&self) -> Result<String, serde_json::Error> {
        let snapshot = self.snapshot();
        serde_json::to_string(&snapshot)
    }

    /// Start timing an operation
    pub fn start_timer(&self, operation_name: &'static str) -> OperationTimer {
        OperationTimer {
            start: Instant::now(),
            operation_name,
        }
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationTimer {
    /// Get elapsed time in milliseconds
    pub fn elapsed_ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }

    /// Get elapsed time in microseconds
    pub fn elapsed_micros(&self) -> u128 {
        self.start.elapsed().as_micros()
    }

    /// Get operation name
    pub fn operation_name(&self) -> &'static str {
        self.operation_name
    }
}

impl MetricsSnapshot {
    /// Format as human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "Server Metrics (uptime: {}s)\n\
             Connections: {} total, {} active, {} failed, {} rejected\n\
             Auth: {} attempts, {} success ({:.1}% success rate), {} failures, {} rate-limited\n\
             Files: {} opens, {} reads, {} writes, {} closes, {} removes, {} renames\n\
             Dirs: {} opens, {} reads, {} creates, {} removes\n\
             Advanced: {} stat, {} setstat, {} symlink, {} readlink\n\
             Data: {} bytes read, {} bytes written ({} total)\n\
             Errors: {} total ({} protocol, {} permission, {} not_found, {} io, {} timeout)\n\
             Performance: {} total ops, {:.2} ops/sec",
            self.uptime_seconds,
            self.total_connections, self.active_connections, self.failed_connections, self.rejected_connections,
            self.auth_attempts, self.auth_successes, self.auth_success_rate, self.auth_failures, self.rate_limited_attempts,
            self.file_opens, self.file_reads, self.file_writes, self.file_closes, self.file_removes, self.file_renames,
            self.dir_opens, self.dir_reads, self.dir_creates, self.dir_removes,
            self.stat_operations, self.setstat_operations, self.symlink_operations, self.readlink_operations,
            self.bytes_read, self.bytes_written, self.total_bytes,
            self.total_errors, self.protocol_errors, self.permission_denied, self.file_not_found, self.io_errors, self.timeout_errors,
            self.total_operations, self.operations_per_second
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = Metrics::new();
        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.total_connections, 0);
        assert_eq!(snapshot.active_connections, 0);
        assert_eq!(snapshot.total_operations, 0);
    }

    #[test]
    fn test_connection_metrics() {
        let metrics = Metrics::new();

        metrics.record_connection();
        metrics.record_connection();
        metrics.record_connection_close();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_connections, 2);
        assert_eq!(snapshot.active_connections, 1);
    }

    #[test]
    fn test_auth_metrics() {
        let metrics = Metrics::new();

        metrics.record_auth_attempt();
        metrics.record_auth_success();
        metrics.record_auth_attempt();
        metrics.record_auth_failure();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.auth_attempts, 2);
        assert_eq!(snapshot.auth_successes, 1);
        assert_eq!(snapshot.auth_failures, 1);
        assert_eq!(snapshot.auth_success_rate, 50.0);
    }

    #[test]
    fn test_file_operation_metrics() {
        let metrics = Metrics::new();

        metrics.record_file_open();
        metrics.record_file_read(1024);
        metrics.record_file_write(2048);
        metrics.record_file_close();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.file_opens, 1);
        assert_eq!(snapshot.file_reads, 1);
        assert_eq!(snapshot.file_writes, 1);
        assert_eq!(snapshot.file_closes, 1);
        assert_eq!(snapshot.bytes_read, 1024);
        assert_eq!(snapshot.bytes_written, 2048);
        assert_eq!(snapshot.total_operations, 4);
    }

    #[test]
    fn test_error_metrics() {
        let metrics = Metrics::new();

        metrics.record_protocol_error();
        metrics.record_permission_denied();
        metrics.record_file_not_found();
        metrics.record_io_error();
        metrics.record_timeout_error();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.protocol_errors, 1);
        assert_eq!(snapshot.permission_denied, 1);
        assert_eq!(snapshot.file_not_found, 1);
        assert_eq!(snapshot.io_errors, 1);
        assert_eq!(snapshot.timeout_errors, 1);
        assert_eq!(snapshot.total_errors, 5);
    }

    #[test]
    fn test_json_export() {
        let metrics = Metrics::new();
        metrics.record_file_open();
        metrics.record_file_read(100);

        let json = metrics.to_json().expect("JSON serialization failed");
        assert!(json.contains("\"file_opens\": 1"));
        assert!(json.contains("\"bytes_read\": 100"));
    }

    #[test]
    fn test_operation_timer() {
        let metrics = Metrics::new();
        let timer = metrics.start_timer("test_operation");

        std::thread::sleep(std::time::Duration::from_millis(10));

        assert!(timer.elapsed_ms() >= 10);
        assert_eq!(timer.operation_name(), "test_operation");
    }

    #[test]
    fn test_metrics_summary() {
        let metrics = Metrics::new();
        metrics.record_connection();
        metrics.record_file_open();

        let snapshot = metrics.snapshot();
        let summary = snapshot.summary();

        assert!(summary.contains("1 total"));
        assert!(summary.contains("1 opens"));
    }
}
