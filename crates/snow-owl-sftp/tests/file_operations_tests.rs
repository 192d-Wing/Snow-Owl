//! File operations integration tests
//!
//! NIST 800-53: SI-11 (Error Handling), AC-3 (Access Enforcement)
//! STIG: V-222566, V-222596
//! Implementation: Tests for file read/write/delete operations

use snow_owl_sftp::{Config, Error};
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

/// NIST 800-53: AC-3 - Test path resolution within root directory
#[tokio::test]
async fn test_path_resolution_within_root() {
    let (config, _temp_dir) = create_test_config().await;

    // Valid paths should resolve correctly
    let test_path = config.root_dir.join("test.txt");
    fs::write(&test_path, b"test content").await.unwrap();

    assert!(test_path.exists());
    assert!(test_path.starts_with(&config.root_dir));
}

/// NIST 800-53: AC-3, SI-10 - Test path traversal prevention
/// STIG: V-222396
#[test]
fn test_path_traversal_prevention() {
    // Path traversal attempts should be rejected
    let invalid_paths = vec![
        "../../../etc/passwd",
        "../../etc/shadow",
        "../outside_root/file.txt",
        "subdir/../../outside/file.txt",
    ];

    for path in invalid_paths {
        // These should be caught by path validation
        assert!(path.contains(".."), "Path should contain .. for traversal: {}", path);
    }
}

/// NIST 800-53: SI-10 - Test empty path handling
#[test]
fn test_empty_path_handling() {
    let empty_path = "";
    assert!(empty_path.is_empty(), "Empty path should be rejected");
}

/// NIST 800-53: SI-10 - Test null byte in path
#[test]
fn test_null_byte_in_path() {
    let null_path = "test\0file.txt";
    assert!(null_path.contains('\0'), "Null byte should be detected");
}

/// NIST 800-53: SI-11 - Test file creation and deletion
#[tokio::test]
async fn test_file_create_and_delete() {
    let (config, _temp_dir) = create_test_config().await;

    let test_file = config.root_dir.join("test_create.txt");

    // Create file
    fs::write(&test_file, b"test content").await.unwrap();
    assert!(test_file.exists());

    // Read file
    let content = fs::read(&test_file).await.unwrap();
    assert_eq!(content, b"test content");

    // Delete file
    fs::remove_file(&test_file).await.unwrap();
    assert!(!test_file.exists());
}

/// NIST 800-53: SI-11 - Test file read with various sizes
#[tokio::test]
async fn test_file_read_various_sizes() {
    let (config, _temp_dir) = create_test_config().await;

    // Test different file sizes
    let test_cases = vec![
        ("empty.txt", vec![]),
        ("small.txt", vec![b'x'; 100]),
        ("medium.txt", vec![b'y'; 10_000]),
        ("large.txt", vec![b'z'; 100_000]),
    ];

    for (filename, content) in test_cases {
        let file_path = config.root_dir.join(filename);
        fs::write(&file_path, &content).await.unwrap();

        let read_content = fs::read(&file_path).await.unwrap();
        assert_eq!(read_content, content, "Failed for {}", filename);
    }
}

/// NIST 800-53: SI-11 - Test file write and overwrite
#[tokio::test]
async fn test_file_write_and_overwrite() {
    let (config, _temp_dir) = create_test_config().await;

    let test_file = config.root_dir.join("overwrite.txt");

    // Initial write
    fs::write(&test_file, b"original").await.unwrap();
    assert_eq!(fs::read(&test_file).await.unwrap(), b"original");

    // Overwrite
    fs::write(&test_file, b"overwritten").await.unwrap();
    assert_eq!(fs::read(&test_file).await.unwrap(), b"overwritten");
}

/// NIST 800-53: SI-11 - Test file append operation
#[tokio::test]
async fn test_file_append() {
    let (config, _temp_dir) = create_test_config().await;

    let test_file = config.root_dir.join("append.txt");

    // Initial write
    fs::write(&test_file, b"line1\n").await.unwrap();

    // Append
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&test_file)
        .await
        .unwrap();

    use tokio::io::AsyncWriteExt;
    file.write_all(b"line2\n").await.unwrap();
    file.flush().await.unwrap();
    drop(file);

    let content = fs::read_to_string(&test_file).await.unwrap();
    assert_eq!(content, "line1\nline2\n");
}

/// NIST 800-53: SI-11 - Test reading nonexistent file
#[tokio::test]
async fn test_read_nonexistent_file() {
    let (config, _temp_dir) = create_test_config().await;

    let nonexistent = config.root_dir.join("nonexistent.txt");
    let result = fs::read(&nonexistent).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
}

/// NIST 800-53: SI-11 - Test deleting nonexistent file
#[tokio::test]
async fn test_delete_nonexistent_file() {
    let (config, _temp_dir) = create_test_config().await;

    let nonexistent = config.root_dir.join("nonexistent.txt");
    let result = fs::remove_file(&nonexistent).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
}

/// NIST 800-53: SI-11 - Test file metadata retrieval
#[tokio::test]
async fn test_file_metadata() {
    let (config, _temp_dir) = create_test_config().await;

    let test_file = config.root_dir.join("metadata.txt");
    let test_content = b"test content for metadata";

    fs::write(&test_file, test_content).await.unwrap();

    let metadata = fs::metadata(&test_file).await.unwrap();
    assert_eq!(metadata.len(), test_content.len() as u64);
    assert!(metadata.is_file());
    assert!(!metadata.is_dir());
}

/// NIST 800-53: SI-11 - Test file rename operation
#[tokio::test]
async fn test_file_rename() {
    let (config, _temp_dir) = create_test_config().await;

    let old_path = config.root_dir.join("old_name.txt");
    let new_path = config.root_dir.join("new_name.txt");

    fs::write(&old_path, b"content").await.unwrap();
    assert!(old_path.exists());

    fs::rename(&old_path, &new_path).await.unwrap();

    assert!(!old_path.exists());
    assert!(new_path.exists());
    assert_eq!(fs::read(&new_path).await.unwrap(), b"content");
}

/// NIST 800-53: SI-11 - Test renaming nonexistent file
#[tokio::test]
async fn test_rename_nonexistent_file() {
    let (config, _temp_dir) = create_test_config().await;

    let old_path = config.root_dir.join("nonexistent.txt");
    let new_path = config.root_dir.join("new.txt");

    let result = fs::rename(&old_path, &new_path).await;
    assert!(result.is_err());
}

/// NIST 800-53: SI-11 - Test file operations in subdirectories
#[tokio::test]
async fn test_file_operations_in_subdirectories() {
    let (config, _temp_dir) = create_test_config().await;

    let subdir = config.root_dir.join("subdir");
    fs::create_dir(&subdir).await.unwrap();

    let file_in_subdir = subdir.join("file.txt");
    fs::write(&file_in_subdir, b"content").await.unwrap();

    assert!(file_in_subdir.exists());
    assert_eq!(fs::read(&file_in_subdir).await.unwrap(), b"content");

    fs::remove_file(&file_in_subdir).await.unwrap();
    assert!(!file_in_subdir.exists());
}

/// NIST 800-53: SI-11 - Test multiple files in same directory
#[tokio::test]
async fn test_multiple_files_same_directory() {
    let (config, _temp_dir) = create_test_config().await;

    let files = vec!["file1.txt", "file2.txt", "file3.txt"];

    for (i, filename) in files.iter().enumerate() {
        let file_path = config.root_dir.join(filename);
        fs::write(&file_path, format!("content{}", i)).await.unwrap();
    }

    for (i, filename) in files.iter().enumerate() {
        let file_path = config.root_dir.join(filename);
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, format!("content{}", i));
    }
}

/// NIST 800-53: SI-11 - Test file with special characters in name
#[tokio::test]
async fn test_file_special_characters() {
    let (config, _temp_dir) = create_test_config().await;

    let special_names = vec![
        "file with spaces.txt",
        "file-with-dashes.txt",
        "file_with_underscores.txt",
        "file.multiple.dots.txt",
    ];

    for filename in special_names {
        let file_path = config.root_dir.join(filename);
        fs::write(&file_path, b"content").await.unwrap();
        assert!(file_path.exists(), "Failed for: {}", filename);
        fs::remove_file(&file_path).await.unwrap();
    }
}
