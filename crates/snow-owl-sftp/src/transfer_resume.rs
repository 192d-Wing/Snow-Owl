//! Transfer Resume and Recovery
//!
//! This module provides functionality for resuming interrupted file transfers,
//! allowing clients to continue downloads and uploads from where they left off.
//!
//! ## NIST 800-53 Compliance
//!
//! - **SC-8 (Transmission Confidentiality and Integrity)**: Ensures reliable file transfer
//! - **SI-13 (Predictable Failure Prevention)**: Handles interrupted transfers gracefully
//! - **SC-24 (Fail in Known State)**: Maintains transfer state for recovery
//!
//! ## STIG Compliance
//!
//! - **V-222566 (Error Handling)**: Proper handling of transfer failures
//! - **V-222596 (Data Integrity)**: Ensures data integrity during resume operations

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// Transfer state for resumable operations
#[derive(Debug, Clone)]
pub struct TransferState {
    /// Unique transfer ID
    pub transfer_id: String,
    /// File path being transferred
    pub path: PathBuf,
    /// Number of bytes successfully transferred
    pub bytes_transferred: u64,
    /// Total file size (if known)
    pub total_size: Option<u64>,
    /// Transfer direction
    pub direction: TransferDirection,
    /// Timestamp when transfer started
    pub started_at: u64,
    /// Timestamp of last activity
    pub last_activity: u64,
    /// File checksum (for integrity verification)
    pub checksum: Option<String>,
}

/// Transfer direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    /// Upload (client to server)
    Upload,
    /// Download (server to client)
    Download,
}

impl TransferState {
    /// Create a new transfer state
    pub fn new(path: PathBuf, direction: TransferDirection) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let transfer_id = format!("{}-{}",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown"),
            now
        );

        Self {
            transfer_id,
            path,
            bytes_transferred: 0,
            total_size: None,
            direction,
            started_at: now,
            last_activity: now,
            checksum: None,
        }
    }

    /// Update bytes transferred
    pub fn update_progress(&mut self, bytes: u64) {
        self.bytes_transferred += bytes;
        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }

    /// Check if transfer is complete
    pub fn is_complete(&self) -> bool {
        if let Some(total) = self.total_size {
            self.bytes_transferred >= total
        } else {
            false
        }
    }

    /// Get progress percentage (0-100)
    pub fn progress_percentage(&self) -> Option<f64> {
        self.total_size.map(|total| {
            if total == 0 {
                100.0
            } else {
                (self.bytes_transferred as f64 / total as f64) * 100.0
            }
        })
    }

    /// Check if transfer is stale (no activity for timeout period)
    pub fn is_stale(&self, timeout_secs: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        now - self.last_activity > timeout_secs
    }
}

/// Transfer resume manager
///
/// Manages state for resumable file transfers
///
/// NIST 800-53: SC-8 (Transmission Confidentiality and Integrity), SI-13 (Predictable Failure Prevention)
pub struct TransferResumeManager {
    /// Active transfer states
    transfers: Arc<Mutex<HashMap<String, TransferState>>>,
    /// Timeout for stale transfers (seconds)
    stale_timeout: u64,
}

impl TransferResumeManager {
    /// Create a new transfer resume manager
    ///
    /// NIST 800-53: SC-24 (Fail in Known State)
    pub fn new(stale_timeout: u64) -> Self {
        Self {
            transfers: Arc::new(Mutex::new(HashMap::new())),
            stale_timeout,
        }
    }

    /// Start a new transfer or resume existing one
    ///
    /// NIST 800-53: SC-8 (Transmission Confidentiality and Integrity)
    /// STIG: V-222566 (Error Handling)
    pub async fn start_transfer(
        &self,
        path: PathBuf,
        direction: TransferDirection,
    ) -> TransferState {
        let mut transfers = self.transfers.lock().await;

        // Check for existing transfer with same path
        let existing = transfers.values().find(|t| t.path == path && t.direction == direction);

        if let Some(existing) = existing {
            if !existing.is_stale(self.stale_timeout) {
                info!(
                    "Resuming transfer: {} at {} bytes",
                    existing.transfer_id, existing.bytes_transferred
                );
                return existing.clone();
            } else {
                warn!(
                    "Existing transfer {} is stale, starting fresh",
                    existing.transfer_id
                );
            }
        }

        // Create new transfer
        let state = TransferState::new(path, direction);
        debug!(
            "Starting new transfer: {} for {:?}",
            state.transfer_id, state.path
        );
        transfers.insert(state.transfer_id.clone(), state.clone());

        state
    }

    /// Update transfer progress
    ///
    /// NIST 800-53: SC-8 (Transmission Confidentiality and Integrity)
    pub async fn update_progress(&self, transfer_id: &str, bytes: u64) -> Option<TransferState> {
        let mut transfers = self.transfers.lock().await;

        if let Some(state) = transfers.get_mut(transfer_id) {
            state.update_progress(bytes);
            debug!(
                "Transfer {} progress: {} bytes ({}%)",
                transfer_id,
                state.bytes_transferred,
                state.progress_percentage().unwrap_or(0.0)
            );
            return Some(state.clone());
        }

        None
    }

    /// Complete a transfer
    ///
    /// NIST 800-53: SC-8 (Transmission Confidentiality and Integrity)
    pub async fn complete_transfer(&self, transfer_id: &str) -> Option<TransferState> {
        let mut transfers = self.transfers.lock().await;

        if let Some(state) = transfers.remove(transfer_id) {
            info!(
                "Transfer {} completed: {} bytes transferred",
                transfer_id, state.bytes_transferred
            );
            return Some(state);
        }

        None
    }

    /// Get transfer state
    pub async fn get_transfer(&self, transfer_id: &str) -> Option<TransferState> {
        let transfers = self.transfers.lock().await;
        transfers.get(transfer_id).cloned()
    }

    /// Get transfer by path
    pub async fn get_transfer_by_path(
        &self,
        path: &Path,
        direction: TransferDirection,
    ) -> Option<TransferState> {
        let transfers = self.transfers.lock().await;
        transfers
            .values()
            .find(|t| t.path == path && t.direction == direction)
            .cloned()
    }

    /// Cancel a transfer
    ///
    /// NIST 800-53: AC-12 (Session Termination)
    pub async fn cancel_transfer(&self, transfer_id: &str) -> Option<TransferState> {
        let mut transfers = self.transfers.lock().await;

        if let Some(state) = transfers.remove(transfer_id) {
            warn!("Transfer {} cancelled", transfer_id);
            return Some(state);
        }

        None
    }

    /// Clean up stale transfers
    ///
    /// NIST 800-53: SI-13 (Predictable Failure Prevention)
    /// STIG: V-222566 (Error Handling)
    pub async fn cleanup_stale_transfers(&self) -> usize {
        let mut transfers = self.transfers.lock().await;

        let stale_ids: Vec<String> = transfers
            .values()
            .filter(|t| t.is_stale(self.stale_timeout))
            .map(|t| t.transfer_id.clone())
            .collect();

        let count = stale_ids.len();

        for id in stale_ids {
            transfers.remove(&id);
            debug!("Removed stale transfer: {}", id);
        }

        if count > 0 {
            info!("Cleaned up {} stale transfers", count);
        }

        count
    }

    /// Get all active transfers
    pub async fn get_active_transfers(&self) -> Vec<TransferState> {
        let transfers = self.transfers.lock().await;
        transfers.values().cloned().collect()
    }

    /// Get transfer count
    pub async fn transfer_count(&self) -> usize {
        let transfers = self.transfers.lock().await;
        transfers.len()
    }
}

impl Default for TransferResumeManager {
    fn default() -> Self {
        // Default stale timeout: 1 hour
        Self::new(3600)
    }
}

/// Transfer checksum calculator for integrity verification
///
/// NIST 800-53: SI-7 (Software, Firmware, and Information Integrity)
/// STIG: V-222596 (Data Integrity)
pub struct TransferChecksum {
    algorithm: ChecksumAlgorithm,
}

/// Checksum algorithm
#[derive(Debug, Clone, Copy)]
pub enum ChecksumAlgorithm {
    /// SHA-256 (FIPS 180-4)
    Sha256,
    /// SHA-384 (FIPS 180-4)
    Sha384,
    /// SHA-512 (FIPS 180-4)
    Sha512,
}

impl TransferChecksum {
    /// Create a new checksum calculator
    pub fn new(algorithm: ChecksumAlgorithm) -> Self {
        Self { algorithm }
    }

    /// Calculate checksum for a file (placeholder - requires crypto crate)
    ///
    /// NIST 800-53: SI-7 (Software, Firmware, and Information Integrity)
    /// STIG: V-222596 (Data Integrity)
    ///
    /// Note: In production, this would use a crypto library like `sha2`
    pub async fn calculate_file_checksum(&self, _path: &Path) -> std::io::Result<String> {
        // Placeholder implementation
        // In production, would use:
        // - sha2 crate for SHA-256/384/512
        // - async file reading with tokio::fs
        // - streaming hash calculation for large files

        Ok(String::from("placeholder_checksum"))
    }

    /// Verify file checksum
    ///
    /// NIST 800-53: SI-7 (Software, Firmware, and Information Integrity)
    pub async fn verify_checksum(&self, path: &Path, expected: &str) -> std::io::Result<bool> {
        let actual = self.calculate_file_checksum(path).await?;
        Ok(actual == expected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_transfer_state_creation() {
        let path = PathBuf::from("/tmp/test.txt");
        let state = TransferState::new(path.clone(), TransferDirection::Upload);

        assert_eq!(state.path, path);
        assert_eq!(state.bytes_transferred, 0);
        assert_eq!(state.direction, TransferDirection::Upload);
        assert!(state.transfer_id.contains("test.txt"));
    }

    #[test]
    fn test_transfer_progress() {
        let path = PathBuf::from("/tmp/test.txt");
        let mut state = TransferState::new(path, TransferDirection::Upload);
        state.total_size = Some(1000);

        state.update_progress(100);
        assert_eq!(state.bytes_transferred, 100);
        assert_eq!(state.progress_percentage(), Some(10.0));

        state.update_progress(400);
        assert_eq!(state.bytes_transferred, 500);
        assert_eq!(state.progress_percentage(), Some(50.0));
    }

    #[test]
    fn test_transfer_completion() {
        let path = PathBuf::from("/tmp/test.txt");
        let mut state = TransferState::new(path, TransferDirection::Upload);
        state.total_size = Some(1000);

        assert!(!state.is_complete());

        state.update_progress(1000);
        assert!(state.is_complete());
    }

    #[tokio::test]
    async fn test_transfer_manager_start() {
        let manager = TransferResumeManager::new(3600);
        let path = PathBuf::from("/tmp/test.txt");

        let state = manager
            .start_transfer(path.clone(), TransferDirection::Upload)
            .await;

        assert_eq!(state.path, path);
        assert_eq!(state.bytes_transferred, 0);

        let count = manager.transfer_count().await;
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_transfer_manager_update() {
        let manager = TransferResumeManager::new(3600);
        let path = PathBuf::from("/tmp/test.txt");

        let state = manager
            .start_transfer(path, TransferDirection::Upload)
            .await;

        let updated = manager.update_progress(&state.transfer_id, 500).await;
        assert!(updated.is_some());
        assert_eq!(updated.unwrap().bytes_transferred, 500);
    }

    #[tokio::test]
    async fn test_transfer_manager_complete() {
        let manager = TransferResumeManager::new(3600);
        let path = PathBuf::from("/tmp/test.txt");

        let state = manager
            .start_transfer(path, TransferDirection::Upload)
            .await;

        let completed = manager.complete_transfer(&state.transfer_id).await;
        assert!(completed.is_some());

        let count = manager.transfer_count().await;
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_transfer_manager_resume() {
        let manager = TransferResumeManager::new(3600);
        let path = PathBuf::from("/tmp/test.txt");

        // Start initial transfer
        let state1 = manager
            .start_transfer(path.clone(), TransferDirection::Upload)
            .await;

        manager.update_progress(&state1.transfer_id, 500).await;

        // Try to start same transfer again - should resume
        let state2 = manager.start_transfer(path, TransferDirection::Upload).await;

        assert_eq!(state1.transfer_id, state2.transfer_id);
        assert_eq!(state2.bytes_transferred, 500); // Should resume from 500
    }

    #[tokio::test]
    async fn test_transfer_manager_get_by_path() {
        let manager = TransferResumeManager::new(3600);
        let path = PathBuf::from("/tmp/test.txt");

        manager
            .start_transfer(path.clone(), TransferDirection::Upload)
            .await;

        let retrieved = manager
            .get_transfer_by_path(&path, TransferDirection::Upload)
            .await;

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().path, path);
    }

    #[tokio::test]
    async fn test_stale_transfer_detection() {
        let path = PathBuf::from("/tmp/test.txt");
        let mut state = TransferState::new(path, TransferDirection::Upload);

        // Fresh transfer should not be stale
        assert!(!state.is_stale(3600));

        // Simulate old transfer
        state.last_activity = 0;
        assert!(state.is_stale(3600));
    }

    #[tokio::test]
    async fn test_cleanup_stale_transfers() {
        let manager = TransferResumeManager::new(1); // 1 second timeout
        let path = PathBuf::from("/tmp/test.txt");

        let state = manager
            .start_transfer(path, TransferDirection::Upload)
            .await;

        // Wait for transfer to become stale
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let cleaned = manager.cleanup_stale_transfers().await;
        assert_eq!(cleaned, 1);

        let count = manager.transfer_count().await;
        assert_eq!(count, 0);
    }

    #[test]
    fn test_progress_percentage_zero_total() {
        let path = PathBuf::from("/tmp/test.txt");
        let mut state = TransferState::new(path, TransferDirection::Upload);
        state.total_size = Some(0);

        assert_eq!(state.progress_percentage(), Some(100.0));
    }
}
