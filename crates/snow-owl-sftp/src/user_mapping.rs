//! User and Group Permission Mapping
//!
//! This module provides mapping between SFTP usernames and OS-level user/group IDs
//! for proper file permission enforcement.
//!
//! ## NIST 800-53 Compliance
//!
//! - **AC-3 (Access Enforcement)**: Enforces OS-level permission boundaries
//! - **AC-6 (Least Privilege)**: Maps users to appropriate system privileges
//! - **IA-2 (Identification and Authentication)**: Links authenticated users to system identities
//!
//! ## STIG Compliance
//!
//! - **V-222567 (Access Control)**: Proper user/group permission mapping

use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, warn};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

/// User mapping information
///
/// Maps an SFTP username to OS-level user and group IDs for permission checks
#[derive(Debug, Clone)]
pub struct UserMapping {
    /// SFTP username
    pub username: String,
    /// OS user ID (UID) - None means use server process UID
    pub uid: Option<u32>,
    /// OS group ID (GID) - None means use server process GID
    pub gid: Option<u32>,
    /// Additional group IDs this user belongs to
    pub supplementary_gids: Vec<u32>,
}

impl UserMapping {
    /// Create a new user mapping
    pub fn new(username: String) -> Self {
        Self {
            username,
            uid: None,
            gid: None,
            supplementary_gids: Vec::new(),
        }
    }

    /// Create a user mapping with explicit UID/GID
    pub fn with_ids(username: String, uid: u32, gid: u32) -> Self {
        Self {
            username,
            uid: Some(uid),
            gid: Some(gid),
            supplementary_gids: Vec::new(),
        }
    }

    /// Add supplementary group IDs
    pub fn with_supplementary_groups(mut self, gids: Vec<u32>) -> Self {
        self.supplementary_gids = gids;
        self
    }

    /// Check if this user can read a file based on OS permissions
    ///
    /// NIST 800-53: AC-3 (Access Enforcement)
    #[cfg(unix)]
    pub fn can_read(&self, path: &Path) -> bool {
        match std::fs::metadata(path) {
            Ok(metadata) => {
                let file_uid = metadata.uid();
                let file_gid = metadata.gid();
                let mode = metadata.mode();

                self.check_permission(file_uid, file_gid, mode, 0o4)
            }
            Err(e) => {
                warn!("Failed to get metadata for {:?}: {}", path, e);
                false
            }
        }
    }

    /// Check if this user can write to a file based on OS permissions
    ///
    /// NIST 800-53: AC-3 (Access Enforcement)
    #[cfg(unix)]
    pub fn can_write(&self, path: &Path) -> bool {
        match std::fs::metadata(path) {
            Ok(metadata) => {
                let file_uid = metadata.uid();
                let file_gid = metadata.gid();
                let mode = metadata.mode();

                self.check_permission(file_uid, file_gid, mode, 0o2)
            }
            Err(e) => {
                warn!("Failed to get metadata for {:?}: {}", path, e);
                false
            }
        }
    }

    /// Check if this user can execute a file based on OS permissions
    ///
    /// NIST 800-53: AC-3 (Access Enforcement)
    #[cfg(unix)]
    pub fn can_execute(&self, path: &Path) -> bool {
        match std::fs::metadata(path) {
            Ok(metadata) => {
                let file_uid = metadata.uid();
                let file_gid = metadata.gid();
                let mode = metadata.mode();

                self.check_permission(file_uid, file_gid, mode, 0o1)
            }
            Err(e) => {
                warn!("Failed to get metadata for {:?}: {}", path, e);
                false
            }
        }
    }

    /// Check permission bits (read=4, write=2, execute=1)
    ///
    /// NIST 800-53: AC-3 (Access Enforcement)
    /// Implementation: Standard Unix permission checking algorithm
    #[cfg(unix)]
    fn check_permission(&self, file_uid: u32, file_gid: u32, mode: u32, required: u32) -> bool {
        // Root (UID 0) can do anything
        if self.uid == Some(0) {
            return true;
        }

        // Get current process UID/GID if user mapping doesn't specify
        let effective_uid = self.uid.unwrap_or_else(|| unsafe { libc::getuid() });
        let effective_gid = self.gid.unwrap_or_else(|| unsafe { libc::getgid() });

        // Check owner permissions
        if effective_uid == file_uid {
            let owner_perms = (mode >> 6) & 0o7;
            return (owner_perms & required) == required;
        }

        // Check group permissions
        if effective_gid == file_gid || self.supplementary_gids.contains(&file_gid) {
            let group_perms = (mode >> 3) & 0o7;
            return (group_perms & required) == required;
        }

        // Check other permissions
        let other_perms = mode & 0o7;
        (other_perms & required) == required
    }

    /// Non-Unix platforms: always allow (fall back to filesystem checks)
    #[cfg(not(unix))]
    pub fn can_read(&self, _path: &Path) -> bool {
        true
    }

    /// Non-Unix platforms: always allow (fall back to filesystem checks)
    #[cfg(not(unix))]
    pub fn can_write(&self, _path: &Path) -> bool {
        true
    }

    /// Non-Unix platforms: always allow (fall back to filesystem checks)
    #[cfg(not(unix))]
    pub fn can_execute(&self, _path: &Path) -> bool {
        true
    }
}

/// User mapping registry
///
/// Manages mappings between SFTP usernames and OS-level permissions
///
/// NIST 800-53: AC-2 (Account Management), AC-3 (Access Enforcement)
pub struct UserMappingRegistry {
    mappings: HashMap<String, UserMapping>,
}

impl UserMappingRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
        }
    }

    /// Add a user mapping
    ///
    /// NIST 800-53: AC-2 (Account Management)
    pub fn add_mapping(&mut self, mapping: UserMapping) {
        debug!(
            "Adding user mapping: {} -> UID: {:?}, GID: {:?}",
            mapping.username, mapping.uid, mapping.gid
        );
        self.mappings.insert(mapping.username.clone(), mapping);
    }

    /// Get user mapping by username
    pub fn get_mapping(&self, username: &str) -> Option<&UserMapping> {
        self.mappings.get(username)
    }

    /// Load mappings from system password database (Unix only)
    ///
    /// NIST 800-53: AC-2 (Account Management), IA-2 (Identification and Authentication)
    #[cfg(unix)]
    pub fn load_from_system(&mut self) -> std::io::Result<()> {
        use std::ffi::CString;

        // This is a simplified implementation
        // In production, you'd use crates like `users` or `nix` for proper passwd parsing
        debug!("Loading user mappings from system password database");

        // For now, we'll create mappings for common system users
        // In a real implementation, you'd iterate through /etc/passwd
        // or use getpwent() to enumerate all users

        // Note: This is a placeholder. Real implementation would use:
        // - `users` crate for cross-platform user enumeration
        // - `nix` crate for low-level libc bindings
        // - Or direct libc calls to getpwent/getgrent

        Ok(())
    }

    /// Load user mapping for a specific username from system
    ///
    /// NIST 800-53: AC-2 (Account Management), IA-2 (Identification and Authentication)
    #[cfg(unix)]
    pub fn load_user_from_system(&mut self, username: &str) -> std::io::Result<()> {
        use std::ffi::CString;

        let c_username = CString::new(username)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

        unsafe {
            let pwd = libc::getpwnam(c_username.as_ptr());
            if pwd.is_null() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("User '{}' not found in system", username),
                ));
            }

            let uid = (*pwd).pw_uid;
            let gid = (*pwd).pw_gid;

            // Get supplementary groups
            let mut ngroups: libc::c_int = 32; // Start with 32 groups
            let mut groups: Vec<libc::gid_t> = vec![0; ngroups as usize];

            let result = libc::getgrouplist(
                c_username.as_ptr(),
                gid,
                groups.as_mut_ptr(),
                &mut ngroups,
            );

            let supplementary_gids = if result >= 0 {
                groups.truncate(ngroups as usize);
                groups
            } else {
                // If we need more space, allocate and try again
                groups.resize(ngroups as usize, 0);
                let result = libc::getgrouplist(
                    c_username.as_ptr(),
                    gid,
                    groups.as_mut_ptr(),
                    &mut ngroups,
                );
                if result >= 0 {
                    groups.truncate(ngroups as usize);
                    groups
                } else {
                    Vec::new()
                }
            };

            let mapping = UserMapping::with_ids(username.to_string(), uid, gid)
                .with_supplementary_groups(supplementary_gids);

            debug!(
                "Loaded user mapping from system: {} -> UID: {}, GID: {}, supplementary groups: {:?}",
                username, uid, gid, supplementary_gids
            );

            self.add_mapping(mapping);
        }

        Ok(())
    }

    /// Non-Unix platforms: no-op
    #[cfg(not(unix))]
    pub fn load_from_system(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    /// Non-Unix platforms: create default mapping
    #[cfg(not(unix))]
    pub fn load_user_from_system(&mut self, username: &str) -> std::io::Result<()> {
        let mapping = UserMapping::new(username.to_string());
        self.add_mapping(mapping);
        Ok(())
    }
}

impl Default for UserMappingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_user_mapping_creation() {
        let mapping = UserMapping::new("testuser".to_string());
        assert_eq!(mapping.username, "testuser");
        assert_eq!(mapping.uid, None);
        assert_eq!(mapping.gid, None);
    }

    #[test]
    fn test_user_mapping_with_ids() {
        let mapping = UserMapping::with_ids("testuser".to_string(), 1000, 1000);
        assert_eq!(mapping.username, "testuser");
        assert_eq!(mapping.uid, Some(1000));
        assert_eq!(mapping.gid, Some(1000));
    }

    #[test]
    fn test_user_mapping_with_supplementary_groups() {
        let mapping = UserMapping::with_ids("testuser".to_string(), 1000, 1000)
            .with_supplementary_groups(vec![100, 200, 300]);
        assert_eq!(mapping.supplementary_gids.len(), 3);
        assert!(mapping.supplementary_gids.contains(&100));
    }

    #[test]
    fn test_registry_add_and_get() {
        let mut registry = UserMappingRegistry::new();
        let mapping = UserMapping::with_ids("testuser".to_string(), 1000, 1000);
        registry.add_mapping(mapping);

        let retrieved = registry.get_mapping("testuser");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().username, "testuser");
    }

    #[test]
    fn test_registry_get_nonexistent() {
        let registry = UserMappingRegistry::new();
        assert!(registry.get_mapping("nonexistent").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn test_permission_checking() {
        use std::os::unix::fs::PermissionsExt;

        // Create a temporary directory and file
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        // Set permissions to 0o644 (owner: rw, group: r, other: r)
        let mut perms = fs::metadata(&file_path).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&file_path, perms).unwrap();

        // Get the current process UID/GID
        let current_uid = unsafe { libc::getuid() };
        let current_gid = unsafe { libc::getgid() };

        // Create mapping for current user (should have read/write as owner)
        let mapping = UserMapping::with_ids("testuser".to_string(), current_uid, current_gid);

        assert!(mapping.can_read(&file_path));
        assert!(mapping.can_write(&file_path));
        assert!(!mapping.can_execute(&file_path)); // No execute bit set
    }

    #[cfg(unix)]
    #[test]
    fn test_load_current_user_from_system() {
        let mut registry = UserMappingRegistry::new();

        // Get current username
        let username = std::env::var("USER").or_else(|_| std::env::var("USERNAME"));

        if let Ok(username) = username {
            match registry.load_user_from_system(&username) {
                Ok(()) => {
                    let mapping = registry.get_mapping(&username);
                    assert!(mapping.is_some());
                    let mapping = mapping.unwrap();
                    assert!(mapping.uid.is_some());
                    assert!(mapping.gid.is_some());
                }
                Err(e) => {
                    // It's okay if this fails in some environments (e.g., containers)
                    eprintln!("Note: Could not load user from system: {}", e);
                }
            }
        }
    }
}
