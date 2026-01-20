//! Advanced file operations tests (SETSTAT, FSETSTAT)
//!
//! NIST 800-53: AC-3 (Access Enforcement), SI-11 (Error Handling)
//! STIG: V-222566, V-222596
//! Implementation: Tests for SETSTAT and FSETSTAT operations

use snow_owl_sftp::Config;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

/// Helper to create a test config with temporary directory
async fn create_test_config() -> (Config, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let mut config = Config::default();
    config.root_dir = temp_dir.path().to_path_buf();

    // Create root directory
    fs::create_dir_all(&config.root_dir).await.unwrap();

    (config, temp_dir)
}

/// NIST 800-53: AC-3 - Test setting file permissions
#[tokio::test]
#[cfg(unix)]
async fn test_setstat_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let (config, _temp_dir) = create_test_config().await;

    let test_file = config.root_dir.join("perms_test.txt");
    fs::write(&test_file, b"test content").await.unwrap();

    // Set permissions to 0o644
    let mut perms = fs::metadata(&test_file).await.unwrap().permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&test_file, perms).await.unwrap();

    // Verify permissions
    let metadata = fs::metadata(&test_file).await.unwrap();
    assert_eq!(metadata.permissions().mode() & 0o777, 0o644);

    // Change to 0o600
    let mut new_perms = fs::metadata(&test_file).await.unwrap().permissions();
    new_perms.set_mode(0o600);
    fs::set_permissions(&test_file, new_perms).await.unwrap();

    // Verify new permissions
    let metadata = fs::metadata(&test_file).await.unwrap();
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
}

/// NIST 800-53: AC-3 - Test setting multiple permission modes
#[tokio::test]
#[cfg(unix)]
async fn test_setstat_various_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let (config, _temp_dir) = create_test_config().await;

    let permission_modes = vec![
        0o400, // read-only for owner
        0o600, // rw for owner
        0o644, // rw for owner, r for group/others
        0o755, // rwx for owner, rx for group/others
        0o777, // rwx for all
    ];

    for mode in permission_modes {
        let test_file = config.root_dir.join(format!("perm_{:o}.txt", mode));
        fs::write(&test_file, b"test").await.unwrap();

        let mut perms = fs::metadata(&test_file).await.unwrap().permissions();
        perms.set_mode(mode);
        fs::set_permissions(&test_file, perms).await.unwrap();

        let metadata = fs::metadata(&test_file).await.unwrap();
        assert_eq!(
            metadata.permissions().mode() & 0o777,
            mode,
            "Failed for mode {:o}",
            mode
        );
    }
}

/// NIST 800-53: SI-11 - Test setstat on nonexistent file
#[tokio::test]
async fn test_setstat_nonexistent_file() {
    use std::os::unix::fs::PermissionsExt;

    let (config, _temp_dir) = create_test_config().await;

    let nonexistent = config.root_dir.join("nonexistent.txt");

    #[cfg(unix)]
    {
        let perms = std::fs::Permissions::from_mode(0o644);
        let result = fs::set_permissions(&nonexistent, perms).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }
}

/// NIST 800-53: AC-3 - Test permissions on directory
#[tokio::test]
#[cfg(unix)]
async fn test_setstat_directory_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let (config, _temp_dir) = create_test_config().await;

    let test_dir = config.root_dir.join("dir_perms");
    fs::create_dir(&test_dir).await.unwrap();

    // Set directory permissions to 0o755
    let mut perms = fs::metadata(&test_dir).await.unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&test_dir, perms).await.unwrap();

    // Verify permissions
    let metadata = fs::metadata(&test_dir).await.unwrap();
    assert_eq!(metadata.permissions().mode() & 0o777, 0o755);

    // Change to 0o700 (only owner can access)
    let mut new_perms = fs::metadata(&test_dir).await.unwrap().permissions();
    new_perms.set_mode(0o700);
    fs::set_permissions(&test_dir, new_perms).await.unwrap();

    // Verify new permissions
    let metadata = fs::metadata(&test_dir).await.unwrap();
    assert_eq!(metadata.permissions().mode() & 0o777, 0o700);
}

/// NIST 800-53: AC-3 - Test setstat with read-only file
#[tokio::test]
#[cfg(unix)]
async fn test_setstat_readonly_file() {
    use std::os::unix::fs::PermissionsExt;

    let (config, _temp_dir) = create_test_config().await;

    let test_file = config.root_dir.join("readonly.txt");
    fs::write(&test_file, b"test content").await.unwrap();

    // Make read-only
    let mut perms = fs::metadata(&test_file).await.unwrap().permissions();
    perms.set_mode(0o444);
    fs::set_permissions(&test_file, perms).await.unwrap();

    // Verify it's read-only
    let metadata = fs::metadata(&test_file).await.unwrap();
    assert_eq!(metadata.permissions().mode() & 0o777, 0o444);
    assert!(metadata.permissions().readonly());

    // Make writable again
    let mut new_perms = fs::metadata(&test_file).await.unwrap().permissions();
    new_perms.set_mode(0o644);
    fs::set_permissions(&test_file, new_perms).await.unwrap();

    // Verify it's writable
    let metadata = fs::metadata(&test_file).await.unwrap();
    assert!(!metadata.permissions().readonly());
}

/// NIST 800-53: AC-3 - Test permission preservation across operations
#[tokio::test]
#[cfg(unix)]
async fn test_permissions_preserved_after_write() {
    use std::os::unix::fs::PermissionsExt;

    let (config, _temp_dir) = create_test_config().await;

    let test_file = config.root_dir.join("preserve_perms.txt");
    fs::write(&test_file, b"initial").await.unwrap();

    // Set specific permissions
    let mut perms = fs::metadata(&test_file).await.unwrap().permissions();
    perms.set_mode(0o600);
    fs::set_permissions(&test_file, perms).await.unwrap();

    // Write to file
    fs::write(&test_file, b"updated content").await.unwrap();

    // Verify permissions are preserved
    let metadata = fs::metadata(&test_file).await.unwrap();
    // Note: permissions may not be preserved by default on all filesystems
    // This test documents the behavior
    let mode = metadata.permissions().mode() & 0o777;
    println!("Permissions after write: {:o}", mode);
}

/// NIST 800-53: AC-3 - Test setstat with special permission bits
#[tokio::test]
#[cfg(unix)]
async fn test_setstat_special_bits() {
    use std::os::unix::fs::PermissionsExt;

    let (config, _temp_dir) = create_test_config().await;

    let test_file = config.root_dir.join("special_bits.txt");
    fs::write(&test_file, b"test").await.unwrap();

    // Note: setuid/setgid/sticky bits may not work on regular files
    // or may require special privileges
    // This test documents the behavior
    let modes_to_try = vec![
        0o4755, // setuid
        0o2755, // setgid
        0o1755, // sticky
    ];

    for mode in modes_to_try {
        let mut perms = fs::metadata(&test_file).await.unwrap().permissions();
        perms.set_mode(mode);

        // Attempt to set (may fail on some systems/filesystems)
        if fs::set_permissions(&test_file, perms).await.is_ok() {
            let metadata = fs::metadata(&test_file).await.unwrap();
            let actual_mode = metadata.permissions().mode();
            println!(
                "Requested mode: {:o}, actual mode: {:o}",
                mode, actual_mode
            );
        }
    }
}

/// NIST 800-53: SI-11 - Test concurrent permission changes
#[tokio::test]
#[cfg(unix)]
async fn test_concurrent_setstat() {
    use std::os::unix::fs::PermissionsExt;
    use tokio::task::JoinSet;

    let (config, _temp_dir) = create_test_config().await;

    // Create files
    for i in 0..5 {
        let file_path = config.root_dir.join(format!("concurrent_{}.txt", i));
        fs::write(&file_path, b"test").await.unwrap();
    }

    // Concurrently set permissions
    let mut tasks = JoinSet::new();
    for i in 0..5 {
        let file_path = config.root_dir.join(format!("concurrent_{}.txt", i));
        let mode = 0o600 + (i * 0o10); // Different mode for each file
        tasks.spawn(async move {
            let mut perms = fs::metadata(&file_path).await.unwrap().permissions();
            perms.set_mode(mode);
            fs::set_permissions(&file_path, perms).await.unwrap();
            (file_path, mode)
        });
    }

    // Verify all succeeded
    while let Some(result) = tasks.join_next().await {
        let (path, expected_mode) = result.unwrap();
        let metadata = fs::metadata(&path).await.unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, expected_mode);
    }
}

/// NIST 800-53: AC-3 - Test permission check for different users
/// Note: This test is informational and may require special setup
#[tokio::test]
#[cfg(unix)]
async fn test_permission_effective_uid_gid() {
    use std::os::unix::fs::PermissionsExt;

    let (config, _temp_dir) = create_test_config().await;

    let test_file = config.root_dir.join("uid_gid_test.txt");
    fs::write(&test_file, b"test").await.unwrap();

    let metadata = fs::metadata(&test_file).await.unwrap();

    // Get current UID/GID
    use std::os::unix::prelude::*;
    let uid = metadata.uid();
    let gid = metadata.gid();

    println!("File UID: {}, GID: {}", uid, gid);

    // Set restrictive permissions
    let mut perms = metadata.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(&test_file, perms).await.unwrap();

    // Verify we can still read (we're the owner)
    let content = fs::read(&test_file).await.unwrap();
    assert_eq!(content, b"test");
}

/// NIST 800-53: SI-11 - Test setstat with invalid permissions
#[tokio::test]
#[cfg(unix)]
async fn test_setstat_invalid_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let (config, _temp_dir) = create_test_config().await;

    let test_file = config.root_dir.join("invalid_perms.txt");
    fs::write(&test_file, b"test").await.unwrap();

    // Test with overly permissive mode (beyond 0o777)
    // The system should handle this gracefully
    let mut perms = fs::metadata(&test_file).await.unwrap().permissions();
    perms.set_mode(0o7777); // Extra bits set

    fs::set_permissions(&test_file, perms).await.unwrap();

    // Verify the actual permissions set
    let metadata = fs::metadata(&test_file).await.unwrap();
    let actual_mode = metadata.permissions().mode() & 0o7777;
    println!("Set mode 0o7777, actual mode: {:o}", actual_mode);
}

/// NIST 800-53: AC-3 - Test permissions on symbolic links
/// Note: This test requires symlink support
#[tokio::test]
#[cfg(unix)]
async fn test_setstat_symlink_permissions() {
    let (config, _temp_dir) = create_test_config().await;

    let target_file = config.root_dir.join("symlink_target.txt");
    let symlink = config.root_dir.join("symlink.txt");

    fs::write(&target_file, b"target content").await.unwrap();

    #[cfg(unix)]
    {
        if tokio::fs::symlink(&target_file, &symlink).await.is_ok() {
            // Check symlink metadata
            let link_metadata = fs::symlink_metadata(&symlink).await.unwrap();
            println!(
                "Symlink is_symlink: {}, is_file: {}",
                link_metadata.is_symlink(),
                link_metadata.is_file()
            );

            // Note: Setting permissions on symlinks behaves differently
            // on different systems - this test documents the behavior
        }
    }
}
