//! Authentication and security tests
//!
//! NIST 800-53: IA-2 (Authentication), AC-7 (Rate Limiting), AC-10 (Connection Limits)
//! STIG: V-222611, V-222578, V-222601
//! Implementation: Tests for authentication, rate limiting, and connection tracking

use snow_owl_sftp::{AuthorizedKeys, ConnectionTracker, ConnectionTrackerConfig, RateLimitConfig, RateLimiter};
use std::net::IpAddr;
use std::path::PathBuf;

/// NIST 800-53: IA-2 - Test authorized_keys file parsing
#[test]
fn test_authorized_keys_parsing() {
    // This test requires creating a temporary authorized_keys file
    let temp_dir = std::env::temp_dir();
    let auth_keys_path = temp_dir.join("test_authorized_keys");

    // Create a test authorized_keys file
    let test_content = "# Comment line\nssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGPpSXxxx test@example.com\n";
    std::fs::write(&auth_keys_path, test_content).unwrap();

    let mut auth_keys = AuthorizedKeys::new(auth_keys_path.to_string_lossy().to_string());
    let result = auth_keys.load();

    assert!(result.is_ok(), "Should load authorized_keys file");

    // Cleanup
    std::fs::remove_file(&auth_keys_path).ok();
}

/// NIST 800-53: IA-2 - Test authorized_keys with nonexistent file
#[test]
fn test_authorized_keys_nonexistent_file() {
    let mut auth_keys = AuthorizedKeys::new("/nonexistent/path/authorized_keys".to_string());
    let result = auth_keys.load();

    assert!(result.is_err(), "Should fail to load nonexistent file");
}

/// NIST 800-53: IA-2 - Test authorized_keys with empty file
#[test]
fn test_authorized_keys_empty_file() {
    let temp_dir = std::env::temp_dir();
    let auth_keys_path = temp_dir.join("test_authorized_keys_empty");

    std::fs::write(&auth_keys_path, "").unwrap();

    let mut auth_keys = AuthorizedKeys::new(auth_keys_path.to_string_lossy().to_string());
    let result = auth_keys.load();

    assert!(result.is_ok(), "Should handle empty file gracefully");

    // Cleanup
    std::fs::remove_file(&auth_keys_path).ok();
}

/// NIST 800-53: AC-7 - Test rate limiter allows initial attempts
#[tokio::test]
async fn test_rate_limiter_allows_initial_attempts() {
    let config = RateLimitConfig {
        max_attempts: 3,
        window_secs: 60,
        lockout_duration_secs: 60,
    };

    let limiter = RateLimiter::new(config);
    let test_ip: IpAddr = "192.0.2.1".parse().unwrap();

    // First 3 attempts should be allowed
    assert!(limiter.check_allowed(test_ip).await);
    limiter.record_failure(test_ip).await;

    assert!(limiter.check_allowed(test_ip).await);
    limiter.record_failure(test_ip).await;

    assert!(limiter.check_allowed(test_ip).await);
    limiter.record_failure(test_ip).await;
}

/// NIST 800-53: AC-7 - Test rate limiter blocks after max attempts
#[tokio::test]
async fn test_rate_limiter_blocks_after_max_attempts() {
    let config = RateLimitConfig {
        max_attempts: 3,
        window_secs: 60,
        lockout_duration_secs: 60,
    };

    let limiter = RateLimiter::new(config);
    let test_ip: IpAddr = "192.0.2.2".parse().unwrap();

    // Exhaust all attempts
    for _ in 0..3 {
        assert!(limiter.check_allowed(test_ip).await);
        limiter.record_failure(test_ip).await;
    }

    // Next attempt should be blocked
    assert!(!limiter.check_allowed(test_ip).await);
}

/// NIST 800-53: AC-7 - Test rate limiter resets on success
#[tokio::test]
async fn test_rate_limiter_resets_on_success() {
    let config = RateLimitConfig {
        max_attempts: 3,
        window_secs: 60,
        lockout_duration_secs: 60,
    };

    let limiter = RateLimiter::new(config);
    let test_ip: IpAddr = "192.0.2.3".parse().unwrap();

    // Record some failures
    limiter.record_failure(test_ip).await;
    limiter.record_failure(test_ip).await;

    // Success should clear attempts
    limiter.record_success(test_ip).await;

    // Should be allowed again
    assert!(limiter.check_allowed(test_ip).await);
}

/// NIST 800-53: AC-7 - Test rate limiter with different IPs
#[tokio::test]
async fn test_rate_limiter_per_ip_isolation() {
    let config = RateLimitConfig {
        max_attempts: 2,
        window_secs: 60,
        lockout_duration_secs: 60,
    };

    let limiter = RateLimiter::new(config);
    let ip1: IpAddr = "192.0.2.10".parse().unwrap();
    let ip2: IpAddr = "192.0.2.11".parse().unwrap();

    // Block IP1
    limiter.record_failure(ip1).await;
    limiter.record_failure(ip1).await;
    assert!(!limiter.check_allowed(ip1).await);

    // IP2 should still be allowed
    assert!(limiter.check_allowed(ip2).await);
}

/// NIST 800-53: AC-7 - Test rate limiter with IPv6 addresses
#[tokio::test]
async fn test_rate_limiter_ipv6() {
    let config = RateLimitConfig {
        max_attempts: 3,
        window_secs: 60,
        lockout_duration_secs: 60,
    };

    let limiter = RateLimiter::new(config);
    let ipv6: IpAddr = "2001:db8::1".parse().unwrap();

    assert!(limiter.check_allowed(ipv6).await);
    limiter.record_failure(ipv6).await;
    assert!(limiter.check_allowed(ipv6).await);
}

/// NIST 800-53: AC-10 - Test connection tracker allows connections
#[tokio::test]
async fn test_connection_tracker_allows_connections() {
    let config = ConnectionTrackerConfig {
        max_connections_per_user: 5,
    };

    let tracker = ConnectionTracker::new(config);

    // Should allow initial connections
    assert!(tracker.can_connect("user1").await);
    let conn_id = tracker.register_connection("user1".to_string()).await;
    assert!(conn_id.is_some());
}

/// NIST 800-53: AC-10 - Test connection tracker enforces limits
#[tokio::test]
async fn test_connection_tracker_enforces_limit() {
    let config = ConnectionTrackerConfig {
        max_connections_per_user: 2,
    };

    let tracker = ConnectionTracker::new(config);
    let username = "user_limited";

    // Register max connections
    let conn1 = tracker.register_connection(username.to_string()).await;
    let conn2 = tracker.register_connection(username.to_string()).await;

    assert!(conn1.is_some());
    assert!(conn2.is_some());

    // Should not allow more connections
    assert!(!tracker.can_connect(username).await);
    let conn3 = tracker.register_connection(username.to_string()).await;
    assert!(conn3.is_none());
}

/// NIST 800-53: AC-10, AC-12 - Test connection tracker cleanup
#[tokio::test]
async fn test_connection_tracker_cleanup() {
    let config = ConnectionTrackerConfig {
        max_connections_per_user: 2,
    };

    let tracker = ConnectionTracker::new(config);
    let username = "user_cleanup";

    // Register connections
    let conn1 = tracker.register_connection(username.to_string()).await.unwrap();
    let conn2 = tracker.register_connection(username.to_string()).await.unwrap();

    // At limit
    assert!(!tracker.can_connect(username).await);

    // Unregister one connection
    tracker.unregister_connection(username, conn1).await;

    // Should allow new connection now
    assert!(tracker.can_connect(username).await);
    let conn3 = tracker.register_connection(username.to_string()).await;
    assert!(conn3.is_some());
}

/// NIST 800-53: AC-10 - Test connection tracker per-user isolation
#[tokio::test]
async fn test_connection_tracker_per_user_isolation() {
    let config = ConnectionTrackerConfig {
        max_connections_per_user: 1,
    };

    let tracker = ConnectionTracker::new(config);

    // Max out user1
    tracker.register_connection("user1".to_string()).await;
    assert!(!tracker.can_connect("user1").await);

    // user2 should still be allowed
    assert!(tracker.can_connect("user2").await);
    tracker.register_connection("user2".to_string()).await;
}

/// NIST 800-53: AC-10 - Test connection tracker counts
#[tokio::test]
async fn test_connection_tracker_get_count() {
    let config = ConnectionTrackerConfig {
        max_connections_per_user: 10,
    };

    let tracker = ConnectionTracker::new(config);
    let username = "user_count";

    assert_eq!(tracker.get_connection_count(username).await, 0);

    tracker.register_connection(username.to_string()).await;
    assert_eq!(tracker.get_connection_count(username).await, 1);

    tracker.register_connection(username.to_string()).await;
    assert_eq!(tracker.get_connection_count(username).await, 2);
}

/// NIST 800-53: AC-10 - Test connection tracker statistics
#[tokio::test]
async fn test_connection_tracker_statistics() {
    let config = ConnectionTrackerConfig {
        max_connections_per_user: 10,
    };

    let tracker = ConnectionTracker::new(config);

    tracker.register_connection("user1".to_string()).await;
    tracker.register_connection("user1".to_string()).await;
    tracker.register_connection("user2".to_string()).await;

    let (active_users, total_connections) = tracker.get_stats().await;
    assert_eq!(active_users, 2);
    assert_eq!(total_connections, 3);
}

/// NIST 800-53: AC-10 - Test connection tracker with zero limit
#[tokio::test]
async fn test_connection_tracker_zero_limit() {
    let config = ConnectionTrackerConfig {
        max_connections_per_user: 0,
    };

    let tracker = ConnectionTracker::new(config);

    // Should not allow any connections with zero limit
    assert!(!tracker.can_connect("user").await);
}

/// NIST 800-53: AC-12 - Test connection cleanup on unregister
#[tokio::test]
async fn test_connection_cleanup_all() {
    let config = ConnectionTrackerConfig {
        max_connections_per_user: 5,
    };

    let tracker = ConnectionTracker::new(config);
    let username = "user_cleanup_all";

    // Register multiple connections
    let conn1 = tracker.register_connection(username.to_string()).await.unwrap();
    let conn2 = tracker.register_connection(username.to_string()).await.unwrap();
    let conn3 = tracker.register_connection(username.to_string()).await.unwrap();

    assert_eq!(tracker.get_connection_count(username).await, 3);

    // Unregister all
    tracker.unregister_connection(username, conn1).await;
    tracker.unregister_connection(username, conn2).await;
    tracker.unregister_connection(username, conn3).await;

    assert_eq!(tracker.get_connection_count(username).await, 0);
}
