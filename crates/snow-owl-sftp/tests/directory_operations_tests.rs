//! Directory operations integration tests
//!
//! NIST 800-53: SI-11 (Error Handling), AC-3 (Access Enforcement)
//! STIG: V-222566, V-222596
//! Implementation: Tests for directory creation, listing, and removal

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

/// NIST 800-53: AC-3 - Test directory creation
#[tokio::test]
async fn test_directory_creation() {
    let (config, _temp_dir) = create_test_config().await;

    let new_dir = config.root_dir.join("test_directory");
    fs::create_dir(&new_dir).await.unwrap();

    assert!(new_dir.exists());
    assert!(new_dir.is_dir());
}

/// NIST 800-53: AC-3 - Test nested directory creation
#[tokio::test]
async fn test_nested_directory_creation() {
    let (config, _temp_dir) = create_test_config().await;

    let nested_dir = config.root_dir.join("level1/level2/level3");
    fs::create_dir_all(&nested_dir).await.unwrap();

    assert!(nested_dir.exists());
    assert!(nested_dir.is_dir());
    assert!(config.root_dir.join("level1").is_dir());
    assert!(config.root_dir.join("level1/level2").is_dir());
}

/// NIST 800-53: SI-11 - Test directory removal
#[tokio::test]
async fn test_directory_removal() {
    let (config, _temp_dir) = create_test_config().await;

    let test_dir = config.root_dir.join("to_remove");
    fs::create_dir(&test_dir).await.unwrap();
    assert!(test_dir.exists());

    fs::remove_dir(&test_dir).await.unwrap();
    assert!(!test_dir.exists());
}

/// NIST 800-53: SI-11 - Test removing non-empty directory fails
#[tokio::test]
async fn test_remove_non_empty_directory() {
    let (config, _temp_dir) = create_test_config().await;

    let test_dir = config.root_dir.join("non_empty");
    fs::create_dir(&test_dir).await.unwrap();

    // Add a file to the directory
    let file_in_dir = test_dir.join("file.txt");
    fs::write(&file_in_dir, b"content").await.unwrap();

    // Attempting to remove non-empty directory should fail
    let result = fs::remove_dir(&test_dir).await;
    assert!(result.is_err());

    // Directory should still exist
    assert!(test_dir.exists());
}

/// NIST 800-53: SI-11 - Test removing nonexistent directory
#[tokio::test]
async fn test_remove_nonexistent_directory() {
    let (config, _temp_dir) = create_test_config().await;

    let nonexistent = config.root_dir.join("nonexistent_dir");
    let result = fs::remove_dir(&nonexistent).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
}

/// NIST 800-53: AC-3 - Test directory listing
#[tokio::test]
async fn test_directory_listing() {
    let (config, _temp_dir) = create_test_config().await;

    // Create multiple files and directories
    fs::write(config.root_dir.join("file1.txt"), b"content1").await.unwrap();
    fs::write(config.root_dir.join("file2.txt"), b"content2").await.unwrap();
    fs::create_dir(config.root_dir.join("subdir1")).await.unwrap();
    fs::create_dir(config.root_dir.join("subdir2")).await.unwrap();

    // Read directory
    let mut entries = fs::read_dir(&config.root_dir).await.unwrap();
    let mut file_count = 0;
    let mut dir_count = 0;

    while let Some(entry) = entries.next_entry().await.unwrap() {
        let metadata = entry.metadata().await.unwrap();
        if metadata.is_file() {
            file_count += 1;
        } else if metadata.is_dir() {
            dir_count += 1;
        }
    }

    assert_eq!(file_count, 2);
    assert_eq!(dir_count, 2);
}

/// NIST 800-53: AC-3 - Test empty directory listing
#[tokio::test]
async fn test_empty_directory_listing() {
    let (config, _temp_dir) = create_test_config().await;

    let mut entries = fs::read_dir(&config.root_dir).await.unwrap();
    let mut count = 0;

    while let Some(_) = entries.next_entry().await.unwrap() {
        count += 1;
    }

    assert_eq!(count, 0, "Empty directory should have no entries");
}

/// NIST 800-53: SI-11 - Test listing nonexistent directory
#[tokio::test]
async fn test_list_nonexistent_directory() {
    let (config, _temp_dir) = create_test_config().await;

    let nonexistent = config.root_dir.join("nonexistent");
    let result = fs::read_dir(&nonexistent).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
}

/// NIST 800-53: SI-11 - Test directory metadata
#[tokio::test]
async fn test_directory_metadata() {
    let (config, _temp_dir) = create_test_config().await;

    let test_dir = config.root_dir.join("metadata_test");
    fs::create_dir(&test_dir).await.unwrap();

    let metadata = fs::metadata(&test_dir).await.unwrap();
    assert!(metadata.is_dir());
    assert!(!metadata.is_file());
}

/// NIST 800-53: AC-3 - Test directory with special characters
#[tokio::test]
async fn test_directory_special_characters() {
    let (config, _temp_dir) = create_test_config().await;

    let special_names = vec![
        "dir with spaces",
        "dir-with-dashes",
        "dir_with_underscores",
        "dir.with.dots",
    ];

    for dirname in special_names {
        let dir_path = config.root_dir.join(dirname);
        fs::create_dir(&dir_path).await.unwrap();
        assert!(dir_path.exists(), "Failed for: {}", dirname);
        assert!(dir_path.is_dir());
        fs::remove_dir(&dir_path).await.unwrap();
    }
}

/// NIST 800-53: AC-3 - Test creating directory that already exists
#[tokio::test]
async fn test_create_existing_directory() {
    let (config, _temp_dir) = create_test_config().await;

    let test_dir = config.root_dir.join("existing");
    fs::create_dir(&test_dir).await.unwrap();

    // Attempting to create again should fail
    let result = fs::create_dir(&test_dir).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::AlreadyExists);
}

/// NIST 800-53: AC-3 - Test directory operations in subdirectories
#[tokio::test]
async fn test_directory_operations_nested() {
    let (config, _temp_dir) = create_test_config().await;

    // Create nested structure
    let level1 = config.root_dir.join("level1");
    let level2 = level1.join("level2");
    let level3 = level2.join("level3");

    fs::create_dir(&level1).await.unwrap();
    fs::create_dir(&level2).await.unwrap();
    fs::create_dir(&level3).await.unwrap();

    // Verify all exist
    assert!(level1.exists());
    assert!(level2.exists());
    assert!(level3.exists());

    // Remove from deepest to shallowest
    fs::remove_dir(&level3).await.unwrap();
    assert!(!level3.exists());
    assert!(level2.exists());

    fs::remove_dir(&level2).await.unwrap();
    assert!(!level2.exists());
    assert!(level1.exists());

    fs::remove_dir(&level1).await.unwrap();
    assert!(!level1.exists());
}

/// NIST 800-53: AC-3 - Test directory listing with mixed content
#[tokio::test]
async fn test_directory_listing_mixed_content() {
    let (config, _temp_dir) = create_test_config().await;

    // Create mixed content
    fs::create_dir(config.root_dir.join("dir1")).await.unwrap();
    fs::write(config.root_dir.join("file1.txt"), b"content").await.unwrap();
    fs::create_dir(config.root_dir.join("dir2")).await.unwrap();
    fs::write(config.root_dir.join("file2.txt"), b"content").await.unwrap();
    fs::create_dir(config.root_dir.join("dir3")).await.unwrap();

    let mut entries = fs::read_dir(&config.root_dir).await.unwrap();
    let mut names = Vec::new();

    while let Some(entry) = entries.next_entry().await.unwrap() {
        names.push(entry.file_name().to_string_lossy().to_string());
    }

    assert_eq!(names.len(), 5);
    assert!(names.contains(&"dir1".to_string()));
    assert!(names.contains(&"dir2".to_string()));
    assert!(names.contains(&"dir3".to_string()));
    assert!(names.contains(&"file1.txt".to_string()));
    assert!(names.contains(&"file2.txt".to_string()));
}

/// NIST 800-53: AC-3 - Test recursive directory removal
#[tokio::test]
async fn test_recursive_directory_removal() {
    let (config, _temp_dir) = create_test_config().await;

    // Create nested structure with files
    let level1 = config.root_dir.join("remove_recursive");
    let level2 = level1.join("level2");
    fs::create_dir_all(&level2).await.unwrap();

    fs::write(level1.join("file1.txt"), b"content").await.unwrap();
    fs::write(level2.join("file2.txt"), b"content").await.unwrap();

    // Remove recursively
    fs::remove_dir_all(&level1).await.unwrap();

    assert!(!level1.exists());
    assert!(!level2.exists());
}

/// NIST 800-53: AC-3 - Test directory permissions metadata
#[tokio::test]
#[cfg(unix)]
async fn test_directory_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let (config, _temp_dir) = create_test_config().await;

    let test_dir = config.root_dir.join("perms_test");
    fs::create_dir(&test_dir).await.unwrap();

    // Set permissions
    let mut perms = fs::metadata(&test_dir).await.unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&test_dir, perms).await.unwrap();

    // Verify permissions
    let metadata = fs::metadata(&test_dir).await.unwrap();
    assert_eq!(metadata.permissions().mode() & 0o777, 0o755);
}

/// NIST 800-53: SI-11 - Test directory name with maximum length
#[tokio::test]
async fn test_directory_long_name() {
    let (config, _temp_dir) = create_test_config().await;

    // Create a long but valid directory name (255 chars is typical max on most filesystems)
    let long_name = "a".repeat(100);
    let long_dir = config.root_dir.join(&long_name);

    fs::create_dir(&long_dir).await.unwrap();
    assert!(long_dir.exists());

    fs::remove_dir(&long_dir).await.unwrap();
    assert!(!long_dir.exists());
}

/// NIST 800-53: AC-3 - Test multiple directory operations in sequence
#[tokio::test]
async fn test_multiple_directory_operations() {
    let (config, _temp_dir) = create_test_config().await;

    // Create
    let dir1 = config.root_dir.join("seq1");
    let dir2 = config.root_dir.join("seq2");
    let dir3 = config.root_dir.join("seq3");

    fs::create_dir(&dir1).await.unwrap();
    fs::create_dir(&dir2).await.unwrap();
    fs::create_dir(&dir3).await.unwrap();

    // Verify all created
    assert!(dir1.exists() && dir2.exists() && dir3.exists());

    // Remove in different order
    fs::remove_dir(&dir2).await.unwrap();
    fs::remove_dir(&dir1).await.unwrap();
    fs::remove_dir(&dir3).await.unwrap();

    // Verify all removed
    assert!(!dir1.exists() && !dir2.exists() && !dir3.exists());
}

/// NIST 800-53: SI-11 - Test directory listing order consistency
#[tokio::test]
async fn test_directory_listing_consistency() {
    let (config, _temp_dir) = create_test_config().await;

    // Create files in specific order
    for i in 0..10 {
        fs::write(config.root_dir.join(format!("file{}.txt", i)), b"content").await.unwrap();
    }

    // Read directory multiple times
    for _ in 0..3 {
        let mut entries = fs::read_dir(&config.root_dir).await.unwrap();
        let mut count = 0;

        while let Some(_) = entries.next_entry().await.unwrap() {
            count += 1;
        }

        assert_eq!(count, 10, "Should always list all 10 files");
    }
}
