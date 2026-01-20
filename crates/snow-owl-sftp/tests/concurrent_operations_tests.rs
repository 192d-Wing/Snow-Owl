//! Concurrent operations integration tests
//!
//! NIST 800-53: AC-10 (Concurrent Session Control), AC-12 (Session Termination)
//! STIG: V-222601
//! Implementation: Tests for concurrent file operations, connection limits, and rate limiting

use snow_owl_sftp::{Config, ConnectionTracker, ConnectionTrackerConfig, RateLimiter, RateLimitConfig};
use std::net::IpAddr;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::fs;
use tokio::task::JoinSet;

/// Helper to create a test config with temporary directory
async fn create_test_config() -> (Config, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let mut config = Config::default();
    config.root_dir = temp_dir.path().to_path_buf();

    // Create root directory
    fs::create_dir_all(&config.root_dir).await.unwrap();

    (config, temp_dir)
}

/// NIST 800-53: AC-10 - Test concurrent file reads
#[tokio::test]
async fn test_concurrent_file_reads() {
    let (config, _temp_dir) = create_test_config().await;

    // Create test file
    let test_file = config.root_dir.join("concurrent_read.txt");
    let test_content = b"test content for concurrent reads";
    fs::write(&test_file, test_content).await.unwrap();

    // Spawn multiple concurrent read tasks
    let mut tasks = JoinSet::new();
    for _ in 0..10 {
        let file_path = test_file.clone();
        tasks.spawn(async move {
            fs::read(&file_path).await.unwrap()
        });
    }

    // Collect results
    let mut successful_reads = 0;
    while let Some(result) = tasks.join_next().await {
        let content = result.unwrap();
        assert_eq!(content, test_content);
        successful_reads += 1;
    }

    assert_eq!(successful_reads, 10);
}

/// NIST 800-53: AC-10 - Test concurrent file writes to different files
#[tokio::test]
async fn test_concurrent_file_writes_different_files() {
    let (config, _temp_dir) = create_test_config().await;

    // Spawn multiple concurrent write tasks
    let mut tasks = JoinSet::new();
    for i in 0..10 {
        let file_path = config.root_dir.join(format!("concurrent_write_{}.txt", i));
        tasks.spawn(async move {
            fs::write(&file_path, format!("content {}", i)).await.unwrap();
            file_path
        });
    }

    // Wait for all writes to complete
    let mut paths = Vec::new();
    while let Some(result) = tasks.join_next().await {
        paths.push(result.unwrap());
    }

    // Verify all files were created
    assert_eq!(paths.len(), 10);
    for (i, path) in paths.iter().enumerate() {
        assert!(path.exists());
        let content = fs::read_to_string(path).await.unwrap();
        assert_eq!(content, format!("content {}", i));
    }
}

/// NIST 800-53: AC-10 - Test concurrent directory creation
#[tokio::test]
async fn test_concurrent_directory_creation() {
    let (config, _temp_dir) = create_test_config().await;

    // Spawn multiple concurrent directory creation tasks
    let mut tasks = JoinSet::new();
    for i in 0..10 {
        let dir_path = config.root_dir.join(format!("concurrent_dir_{}", i));
        tasks.spawn(async move {
            fs::create_dir(&dir_path).await.unwrap();
            dir_path
        });
    }

    // Wait for all to complete
    let mut dirs = Vec::new();
    while let Some(result) = tasks.join_next().await {
        dirs.push(result.unwrap());
    }

    // Verify all directories were created
    assert_eq!(dirs.len(), 10);
    for dir in dirs {
        assert!(dir.exists());
        assert!(dir.is_dir());
    }
}

/// NIST 800-53: AC-10 - Test concurrent file operations (read + write)
#[tokio::test]
async fn test_concurrent_mixed_file_operations() {
    let (config, _temp_dir) = create_test_config().await;

    // Create initial files for reading
    for i in 0..5 {
        fs::write(
            config.root_dir.join(format!("read_file_{}.txt", i)),
            format!("read content {}", i)
        ).await.unwrap();
    }

    // Spawn mixed read and write tasks
    let mut tasks = JoinSet::new();

    // Read tasks
    for i in 0..5 {
        let file_path = config.root_dir.join(format!("read_file_{}.txt", i));
        tasks.spawn(async move {
            fs::read_to_string(&file_path).await.unwrap()
        });
    }

    // Write tasks
    for i in 0..5 {
        let file_path = config.root_dir.join(format!("write_file_{}.txt", i));
        tasks.spawn(async move {
            fs::write(&file_path, format!("write content {}", i)).await.unwrap();
            "written".to_string()
        });
    }

    // Wait for all to complete
    let mut count = 0;
    while let Some(_) = tasks.join_next().await {
        count += 1;
    }

    assert_eq!(count, 10);
}

/// NIST 800-53: AC-10 - Test ConnectionTracker under concurrent load
#[tokio::test]
async fn test_connection_tracker_concurrent_registrations() {
    let config = ConnectionTrackerConfig {
        max_connections_per_user: 10,
    };

    let tracker = Arc::new(ConnectionTracker::new(config));

    // Spawn concurrent connection registration tasks
    let mut tasks = JoinSet::new();
    for i in 0..20 {
        let tracker_clone = Arc::clone(&tracker);
        let username = format!("user_{}", i % 5); // 5 users, 4 connections each
        tasks.spawn(async move {
            tracker_clone.register_connection(username).await
        });
    }

    // Collect results
    let mut successful = 0;
    let mut failed = 0;
    while let Some(result) = tasks.join_next().await {
        match result.unwrap() {
            Some(_) => successful += 1,
            None => failed += 1,
        }
    }

    // All should succeed since max is 10 and each user only gets 4
    assert_eq!(successful, 20);
    assert_eq!(failed, 0);
}

/// NIST 800-53: AC-10 - Test ConnectionTracker enforces limits under load
#[tokio::test]
async fn test_connection_tracker_limit_enforcement_concurrent() {
    let config = ConnectionTrackerConfig {
        max_connections_per_user: 5,
    };

    let tracker = Arc::new(ConnectionTracker::new(config));
    let username = "limited_user".to_string();

    // Spawn 10 concurrent connection attempts for the same user
    let mut tasks = JoinSet::new();
    for _ in 0..10 {
        let tracker_clone = Arc::clone(&tracker);
        let user = username.clone();
        tasks.spawn(async move {
            tracker_clone.register_connection(user).await
        });
    }

    // Collect results
    let mut successful = 0;
    let mut failed = 0;
    while let Some(result) = tasks.join_next().await {
        match result.unwrap() {
            Some(_) => successful += 1,
            None => failed += 1,
        }
    }

    // Should have exactly 5 successful connections
    assert_eq!(successful, 5);
    assert_eq!(failed, 5);
}

/// NIST 800-53: AC-7 - Test RateLimiter under concurrent authentication attempts
#[tokio::test]
async fn test_rate_limiter_concurrent_attempts() {
    let config = RateLimitConfig {
        max_attempts: 5,
        window_secs: 60,
        lockout_duration_secs: 60,
    };

    let limiter = Arc::new(RateLimiter::new(config));
    let test_ip: IpAddr = "192.0.2.100".parse().unwrap();

    // Spawn concurrent authentication failure tasks
    let mut tasks = JoinSet::new();
    for _ in 0..10 {
        let limiter_clone = Arc::clone(&limiter);
        let ip = test_ip;
        tasks.spawn(async move {
            let allowed = limiter_clone.check_allowed(ip).await;
            if allowed {
                limiter_clone.record_failure(ip).await;
            }
            allowed
        });
    }

    // Collect results
    let mut allowed_count = 0;
    let mut blocked_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result.unwrap() {
            allowed_count += 1;
        } else {
            blocked_count += 1;
        }
    }

    // Should have at most 5 allowed attempts
    assert!(allowed_count <= 5, "Allowed: {}, should be <= 5", allowed_count);
    assert!(blocked_count >= 5, "Blocked: {}, should be >= 5", blocked_count);
}

/// NIST 800-53: AC-7 - Test RateLimiter with multiple IPs concurrently
#[tokio::test]
async fn test_rate_limiter_multiple_ips_concurrent() {
    let config = RateLimitConfig {
        max_attempts: 3,
        window_secs: 60,
        lockout_duration_secs: 60,
    };

    let limiter = Arc::new(RateLimiter::new(config));

    // Spawn concurrent attempts from different IPs
    let mut tasks = JoinSet::new();
    for i in 0..10 {
        let limiter_clone = Arc::clone(&limiter);
        let ip: IpAddr = format!("192.0.2.{}", i + 1).parse().unwrap();
        tasks.spawn(async move {
            // Each IP makes 5 attempts
            let mut allowed = 0;
            for _ in 0..5 {
                if limiter_clone.check_allowed(ip).await {
                    limiter_clone.record_failure(ip).await;
                    allowed += 1;
                }
            }
            allowed
        });
    }

    // Each IP should get exactly 3 attempts
    while let Some(result) = tasks.join_next().await {
        let allowed = result.unwrap();
        assert_eq!(allowed, 3, "Each IP should have exactly 3 allowed attempts");
    }
}

/// NIST 800-53: AC-10, AC-12 - Test connection cleanup under concurrent load
#[tokio::test]
async fn test_connection_cleanup_concurrent() {
    let config = ConnectionTrackerConfig {
        max_connections_per_user: 10,
    };

    let tracker = Arc::new(ConnectionTracker::new(config));
    let username = "cleanup_user".to_string();

    // Register connections
    let mut conn_ids = Vec::new();
    for _ in 0..5 {
        let id = tracker.register_connection(username.clone()).await.unwrap();
        conn_ids.push(id);
    }

    assert_eq!(tracker.get_connection_count(&username).await, 5);

    // Concurrently unregister all connections
    let mut tasks = JoinSet::new();
    for conn_id in conn_ids {
        let tracker_clone = Arc::clone(&tracker);
        let user = username.clone();
        tasks.spawn(async move {
            tracker_clone.unregister_connection(&user, conn_id).await;
        });
    }

    // Wait for all cleanups
    while let Some(_) = tasks.join_next().await {}

    // All connections should be cleaned up
    assert_eq!(tracker.get_connection_count(&username).await, 0);
}

/// NIST 800-53: AC-10 - Test concurrent directory listing
#[tokio::test]
async fn test_concurrent_directory_listing() {
    let (config, _temp_dir) = create_test_config().await;

    // Create files
    for i in 0..20 {
        fs::write(
            config.root_dir.join(format!("file_{}.txt", i)),
            format!("content {}", i)
        ).await.unwrap();
    }

    // Spawn concurrent directory listing tasks
    let mut tasks = JoinSet::new();
    for _ in 0..10 {
        let root_dir = config.root_dir.clone();
        tasks.spawn(async move {
            let mut entries = fs::read_dir(&root_dir).await.unwrap();
            let mut count = 0;
            while let Some(_) = entries.next_entry().await.unwrap() {
                count += 1;
            }
            count
        });
    }

    // All should see the same number of files
    while let Some(result) = tasks.join_next().await {
        let count = result.unwrap();
        assert_eq!(count, 20);
    }
}

/// NIST 800-53: AC-10 - Test concurrent file metadata reads
#[tokio::test]
async fn test_concurrent_metadata_reads() {
    let (config, _temp_dir) = create_test_config().await;

    let test_file = config.root_dir.join("metadata_test.txt");
    let test_content = b"metadata test content";
    fs::write(&test_file, test_content).await.unwrap();

    // Spawn concurrent metadata read tasks
    let mut tasks = JoinSet::new();
    for _ in 0..10 {
        let file_path = test_file.clone();
        tasks.spawn(async move {
            fs::metadata(&file_path).await.unwrap()
        });
    }

    // All should read consistent metadata
    while let Some(result) = tasks.join_next().await {
        let metadata = result.unwrap();
        assert_eq!(metadata.len(), test_content.len() as u64);
        assert!(metadata.is_file());
    }
}

/// NIST 800-53: AC-10 - Test concurrent file renames (to different targets)
#[tokio::test]
async fn test_concurrent_file_renames() {
    let (config, _temp_dir) = create_test_config().await;

    // Create source files
    for i in 0..10 {
        fs::write(
            config.root_dir.join(format!("source_{}.txt", i)),
            format!("content {}", i)
        ).await.unwrap();
    }

    // Spawn concurrent rename tasks
    let mut tasks = JoinSet::new();
    for i in 0..10 {
        let old_path = config.root_dir.join(format!("source_{}.txt", i));
        let new_path = config.root_dir.join(format!("target_{}.txt", i));
        tasks.spawn(async move {
            fs::rename(&old_path, &new_path).await.unwrap();
            new_path
        });
    }

    // Verify all renames completed
    let mut paths = Vec::new();
    while let Some(result) = tasks.join_next().await {
        paths.push(result.unwrap());
    }

    assert_eq!(paths.len(), 10);
    for (i, path) in paths.iter().enumerate() {
        assert!(path.exists());
        let content = fs::read_to_string(path).await.unwrap();
        assert_eq!(content, format!("content {}", i));
    }
}

/// NIST 800-53: AC-10 - Test concurrent file deletions
#[tokio::test]
async fn test_concurrent_file_deletions() {
    let (config, _temp_dir) = create_test_config().await;

    // Create files to delete
    for i in 0..10 {
        fs::write(
            config.root_dir.join(format!("delete_{}.txt", i)),
            b"content"
        ).await.unwrap();
    }

    // Spawn concurrent deletion tasks
    let mut tasks = JoinSet::new();
    for i in 0..10 {
        let file_path = config.root_dir.join(format!("delete_{}.txt", i));
        tasks.spawn(async move {
            fs::remove_file(&file_path).await.unwrap();
            file_path
        });
    }

    // Verify all deletions completed
    let mut paths = Vec::new();
    while let Some(result) = tasks.join_next().await {
        paths.push(result.unwrap());
    }

    assert_eq!(paths.len(), 10);
    for path in paths {
        assert!(!path.exists());
    }
}

/// NIST 800-53: AC-10 - Test stress test with many concurrent operations
#[tokio::test]
async fn test_high_concurrency_stress() {
    let (config, _temp_dir) = create_test_config().await;

    // Create initial files
    for i in 0..50 {
        fs::write(
            config.root_dir.join(format!("stress_{}.txt", i)),
            format!("initial content {}", i)
        ).await.unwrap();
    }

    // Spawn many concurrent mixed operations
    let mut tasks = JoinSet::new();

    // 50 reads
    for i in 0..50 {
        let file_path = config.root_dir.join(format!("stress_{}.txt", i));
        tasks.spawn(async move {
            fs::read(&file_path).await.unwrap();
        });
    }

    // 25 writes
    for i in 0..25 {
        let file_path = config.root_dir.join(format!("new_stress_{}.txt", i));
        tasks.spawn(async move {
            fs::write(&file_path, b"new content").await.unwrap();
        });
    }

    // 25 metadata reads
    for i in 0..25 {
        let file_path = config.root_dir.join(format!("stress_{}.txt", i));
        tasks.spawn(async move {
            fs::metadata(&file_path).await.unwrap();
        });
    }

    // Wait for all operations
    let mut count = 0;
    while let Some(_) = tasks.join_next().await {
        count += 1;
    }

    assert_eq!(count, 100);
}

/// NIST 800-53: AC-10 - Test ConnectionTracker statistics under concurrent load
#[tokio::test]
async fn test_connection_tracker_stats_concurrent() {
    let config = ConnectionTrackerConfig {
        max_connections_per_user: 20,
    };

    let tracker = Arc::new(ConnectionTracker::new(config));

    // Register connections for multiple users concurrently
    let mut tasks = JoinSet::new();
    for user_id in 0..5 {
        for _ in 0..3 {
            let tracker_clone = Arc::clone(&tracker);
            let username = format!("user_{}", user_id);
            tasks.spawn(async move {
                tracker_clone.register_connection(username).await
            });
        }
    }

    // Wait for all registrations
    while let Some(_) = tasks.join_next().await {}

    // Check statistics
    let (active_users, total_connections) = tracker.get_stats().await;
    assert_eq!(active_users, 5);
    assert_eq!(total_connections, 15);
}
