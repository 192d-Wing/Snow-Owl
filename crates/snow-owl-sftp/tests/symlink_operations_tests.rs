//! Symbolic link operations tests
//!
//! NIST 800-53: AC-3 (Access Enforcement), SI-11 (Error Handling)
//! STIG: V-222566, V-222596
//! Implementation: Tests for READLINK and SYMLINK operations

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

/// NIST 800-53: AC-3 - Test creating a symbolic link
#[tokio::test]
#[cfg(unix)]
async fn test_create_symlink() {
    let (config, _temp_dir) = create_test_config().await;

    let target_file = config.root_dir.join("target.txt");
    let symlink = config.root_dir.join("link.txt");

    // Create target file
    fs::write(&target_file, b"target content").await.unwrap();

    // Create symlink
    fs::symlink(&target_file, &symlink).await.unwrap();

    // Verify symlink exists
    assert!(symlink.exists());
    let metadata = fs::symlink_metadata(&symlink).await.unwrap();
    assert!(metadata.is_symlink());
}

/// NIST 800-53: AC-3 - Test reading symlink target
#[tokio::test]
#[cfg(unix)]
async fn test_read_symlink() {
    let (config, _temp_dir) = create_test_config().await;

    let target_file = config.root_dir.join("target.txt");
    let symlink = config.root_dir.join("link.txt");

    fs::write(&target_file, b"content").await.unwrap();
    fs::symlink(&target_file, &symlink).await.unwrap();

    // Read symlink target
    let link_target = fs::read_link(&symlink).await.unwrap();
    assert_eq!(link_target, target_file);
}

/// NIST 800-53: AC-3 - Test reading through symlink
#[tokio::test]
#[cfg(unix)]
async fn test_read_through_symlink() {
    let (config, _temp_dir) = create_test_config().await;

    let target_file = config.root_dir.join("target.txt");
    let symlink = config.root_dir.join("link.txt");

    let test_content = b"test content through symlink";
    fs::write(&target_file, test_content).await.unwrap();
    fs::symlink(&target_file, &symlink).await.unwrap();

    // Read content through symlink
    let content = fs::read(&symlink).await.unwrap();
    assert_eq!(content, test_content);
}

/// NIST 800-53: AC-3 - Test relative symlink
#[tokio::test]
#[cfg(unix)]
async fn test_relative_symlink() {
    let (config, _temp_dir) = create_test_config().await;

    let target_file = config.root_dir.join("target.txt");
    let symlink = config.root_dir.join("link.txt");

    fs::write(&target_file, b"content").await.unwrap();

    // Create relative symlink
    fs::symlink("target.txt", &symlink).await.unwrap();

    // Verify it works
    let content = fs::read(&symlink).await.unwrap();
    assert_eq!(content, b"content");

    // Read the link target
    let link_target = fs::read_link(&symlink).await.unwrap();
    assert_eq!(link_target, PathBuf::from("target.txt"));
}

/// NIST 800-53: AC-3 - Test absolute symlink
#[tokio::test]
#[cfg(unix)]
async fn test_absolute_symlink() {
    let (config, _temp_dir) = create_test_config().await;

    let target_file = config.root_dir.join("target.txt");
    let symlink = config.root_dir.join("link.txt");

    fs::write(&target_file, b"content").await.unwrap();

    // Create absolute symlink
    fs::symlink(&target_file, &symlink).await.unwrap();

    let link_target = fs::read_link(&symlink).await.unwrap();
    assert_eq!(link_target, target_file);
}

/// NIST 800-53: AC-3 - Test symlink to directory
#[tokio::test]
#[cfg(unix)]
async fn test_symlink_to_directory() {
    let (config, _temp_dir) = create_test_config().await;

    let target_dir = config.root_dir.join("target_dir");
    let symlink = config.root_dir.join("link_dir");

    fs::create_dir(&target_dir).await.unwrap();
    fs::write(target_dir.join("file.txt"), b"content").await.unwrap();

    // Create symlink to directory
    fs::symlink(&target_dir, &symlink).await.unwrap();

    // Verify we can access files through the symlink
    let file_through_link = symlink.join("file.txt");
    let content = fs::read(&file_through_link).await.unwrap();
    assert_eq!(content, b"content");
}

/// NIST 800-53: SI-11 - Test symlink to nonexistent target (dangling symlink)
#[tokio::test]
#[cfg(unix)]
async fn test_dangling_symlink() {
    let (config, _temp_dir) = create_test_config().await;

    let nonexistent = config.root_dir.join("nonexistent.txt");
    let symlink = config.root_dir.join("dangling_link.txt");

    // Create symlink to nonexistent file
    fs::symlink(&nonexistent, &symlink).await.unwrap();

    // Symlink exists but target doesn't
    let link_metadata = fs::symlink_metadata(&symlink).await.unwrap();
    assert!(link_metadata.is_symlink());

    // Reading the link should work
    let link_target = fs::read_link(&symlink).await.unwrap();
    assert_eq!(link_target, nonexistent);

    // But following the link should fail
    let result = fs::read(&symlink).await;
    assert!(result.is_err());
}

/// NIST 800-53: SI-11 - Test reading nonexistent symlink
#[tokio::test]
#[cfg(unix)]
async fn test_read_nonexistent_symlink() {
    let (config, _temp_dir) = create_test_config().await;

    let nonexistent_link = config.root_dir.join("nonexistent_link.txt");

    let result = fs::read_link(&nonexistent_link).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
}

/// NIST 800-53: AC-3 - Test symlink chain
#[tokio::test]
#[cfg(unix)]
async fn test_symlink_chain() {
    let (config, _temp_dir) = create_test_config().await;

    let target = config.root_dir.join("target.txt");
    let link1 = config.root_dir.join("link1.txt");
    let link2 = config.root_dir.join("link2.txt");
    let link3 = config.root_dir.join("link3.txt");

    fs::write(&target, b"final content").await.unwrap();
    fs::symlink(&target, &link1).await.unwrap();
    fs::symlink(&link1, &link2).await.unwrap();
    fs::symlink(&link2, &link3).await.unwrap();

    // Should be able to read through the chain
    let content = fs::read(&link3).await.unwrap();
    assert_eq!(content, b"final content");
}

/// NIST 800-53: AC-3 - Test circular symlinks
#[tokio::test]
#[cfg(unix)]
async fn test_circular_symlink() {
    let (config, _temp_dir) = create_test_config().await;

    let link1 = config.root_dir.join("link1.txt");
    let link2 = config.root_dir.join("link2.txt");

    // Create circular symlinks
    fs::symlink(&link2, &link1).await.unwrap();
    fs::symlink(&link1, &link2).await.unwrap();

    // Reading should fail due to too many levels of symlinks
    let result = fs::read(&link1).await;
    assert!(result.is_err());
}

/// NIST 800-53: SI-11 - Test creating symlink that already exists
#[tokio::test]
#[cfg(unix)]
async fn test_create_symlink_already_exists() {
    let (config, _temp_dir) = create_test_config().await;

    let target = config.root_dir.join("target.txt");
    let symlink = config.root_dir.join("link.txt");

    fs::write(&target, b"content").await.unwrap();
    fs::symlink(&target, &symlink).await.unwrap();

    // Try to create again - should fail
    let result = fs::symlink(&target, &symlink).await;
    assert!(result.is_err());
}

/// NIST 800-53: AC-3 - Test symlink in subdirectory
#[tokio::test]
#[cfg(unix)]
async fn test_symlink_in_subdirectory() {
    let (config, _temp_dir) = create_test_config().await;

    let subdir = config.root_dir.join("subdir");
    fs::create_dir(&subdir).await.unwrap();

    let target = subdir.join("target.txt");
    let symlink = subdir.join("link.txt");

    fs::write(&target, b"subdir content").await.unwrap();
    fs::symlink(&target, &symlink).await.unwrap();

    let content = fs::read(&symlink).await.unwrap();
    assert_eq!(content, b"subdir content");
}

/// NIST 800-53: AC-3 - Test symlink with special characters in name
#[tokio::test]
#[cfg(unix)]
async fn test_symlink_special_characters() {
    let (config, _temp_dir) = create_test_config().await;

    let target = config.root_dir.join("target.txt");
    let symlinks = vec![
        "link with spaces.txt",
        "link-with-dashes.txt",
        "link_with_underscores.txt",
    ];

    fs::write(&target, b"content").await.unwrap();

    for link_name in symlinks {
        let symlink = config.root_dir.join(link_name);
        fs::symlink(&target, &symlink).await.unwrap();

        let content = fs::read(&symlink).await.unwrap();
        assert_eq!(content, b"content");
    }
}

/// NIST 800-53: AC-3 - Test removing symlink
#[tokio::test]
#[cfg(unix)]
async fn test_remove_symlink() {
    let (config, _temp_dir) = create_test_config().await;

    let target = config.root_dir.join("target.txt");
    let symlink = config.root_dir.join("link.txt");

    fs::write(&target, b"content").await.unwrap();
    fs::symlink(&target, &symlink).await.unwrap();

    // Remove symlink
    fs::remove_file(&symlink).await.unwrap();

    // Symlink should be gone
    assert!(!symlink.exists());

    // But target should still exist
    assert!(target.exists());
}

/// NIST 800-53: AC-3 - Test symlink metadata
#[tokio::test]
#[cfg(unix)]
async fn test_symlink_metadata() {
    use std::os::unix::fs::MetadataExt;

    let (config, _temp_dir) = create_test_config().await;

    let target = config.root_dir.join("target.txt");
    let symlink = config.root_dir.join("link.txt");

    fs::write(&target, b"test content").await.unwrap();
    fs::symlink(&target, &symlink).await.unwrap();

    // Get symlink metadata (doesn't follow the link)
    let link_metadata = fs::symlink_metadata(&symlink).await.unwrap();
    assert!(link_metadata.is_symlink());
    assert!(!link_metadata.is_file());
    assert!(!link_metadata.is_dir());

    // Get target metadata (follows the link)
    let target_metadata = fs::metadata(&symlink).await.unwrap();
    assert!(!target_metadata.is_symlink());
    assert!(target_metadata.is_file());
    assert_eq!(target_metadata.len(), 12); // "test content" length
}

/// NIST 800-53: AC-3 - Test relative symlink across directories
#[tokio::test]
#[cfg(unix)]
async fn test_relative_symlink_across_directories() {
    let (config, _temp_dir) = create_test_config().await;

    let dir1 = config.root_dir.join("dir1");
    let dir2 = config.root_dir.join("dir2");
    fs::create_dir(&dir1).await.unwrap();
    fs::create_dir(&dir2).await.unwrap();

    let target = dir1.join("target.txt");
    let symlink = dir2.join("link.txt");

    fs::write(&target, b"cross-directory content").await.unwrap();

    // Create relative symlink from dir2 to dir1
    fs::symlink("../dir1/target.txt", &symlink).await.unwrap();

    let content = fs::read(&symlink).await.unwrap();
    assert_eq!(content, b"cross-directory content");
}

/// NIST 800-53: AC-3 - Test symlink permissions don't affect target
#[tokio::test]
#[cfg(unix)]
async fn test_symlink_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let (config, _temp_dir) = create_test_config().await;

    let target = config.root_dir.join("target.txt");
    let symlink = config.root_dir.join("link.txt");

    fs::write(&target, b"content").await.unwrap();

    // Set target permissions
    let mut perms = fs::metadata(&target).await.unwrap().permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&target, perms).await.unwrap();

    fs::symlink(&target, &symlink).await.unwrap();

    // Target permissions should be unchanged
    let target_perms = fs::metadata(&target).await.unwrap().permissions();
    assert_eq!(target_perms.mode() & 0o777, 0o644);
}

/// NIST 800-53: SI-11 - Test concurrent symlink operations
#[tokio::test]
#[cfg(unix)]
async fn test_concurrent_symlink_operations() {
    use tokio::task::JoinSet;

    let (config, _temp_dir) = create_test_config().await;

    // Create target files
    for i in 0..5 {
        let target = config.root_dir.join(format!("target_{}.txt", i));
        fs::write(&target, format!("content {}", i)).await.unwrap();
    }

    // Concurrently create symlinks
    let mut tasks = JoinSet::new();
    for i in 0..5 {
        let target = config.root_dir.join(format!("target_{}.txt", i));
        let symlink = config.root_dir.join(format!("link_{}.txt", i));
        tasks.spawn(async move {
            fs::symlink(&target, &symlink).await.unwrap();
            (symlink, i)
        });
    }

    // Verify all succeeded
    while let Some(result) = tasks.join_next().await {
        let (symlink, i) = result.unwrap();
        let content = fs::read(&symlink).await.unwrap();
        assert_eq!(content, format!("content {}", i).as_bytes());
    }
}
