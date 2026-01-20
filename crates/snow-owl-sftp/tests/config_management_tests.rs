//! Configuration Management Tests
//!
//! Tests for Phase 2.4: Configuration & Management features
//! - Multi-user support with per-user settings
//! - IP whitelist/blacklist
//! - Time-based access restrictions
//! - Operation-based access control

use snow_owl_sftp::{AccessSchedule, Config, UserConfig};
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_user_config_default() {
    let user_config = UserConfig::default();

    assert!(user_config.home_dir.is_none());
    assert_eq!(user_config.bandwidth_limit, 0);
    assert_eq!(user_config.disk_quota, 0);
    assert_eq!(user_config.max_file_size, 0);
    assert!(user_config.max_connections.is_none());
    assert!(user_config.access_schedule.is_none());
    assert!(!user_config.read_only);
    assert!(user_config.allowed_operations.is_none());
    assert!(user_config.denied_operations.is_empty());
}

#[test]
fn test_user_config_with_home_dir() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let mut config = Config::default();
    config.root_dir = temp_dir.path().to_path_buf();

    let mut user_config = UserConfig::default();
    user_config.home_dir = Some(temp_dir.path().to_path_buf());

    config.users.insert("testuser".to_string(), user_config);

    assert!(config.validate().is_ok());

    let retrieved = config.get_user_config("testuser");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().home_dir, Some(temp_dir.path().to_path_buf()));
}

#[test]
fn test_user_config_invalid_home_dir() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let mut config = Config::default();
    config.root_dir = temp_dir.path().to_path_buf();

    let mut user_config = UserConfig::default();
    user_config.home_dir = Some(PathBuf::from("/nonexistent/directory"));

    config.users.insert("testuser".to_string(), user_config);

    assert!(config.validate().is_err());
}

#[test]
fn test_ip_whitelist_empty_allows_all() {
    let config = Config::default();

    let ip1: IpAddr = "192.168.1.1".parse().expect("Failed to parse IP");
    let ip2: IpAddr = "10.0.0.1".parse().expect("Failed to parse IP");

    assert!(config.is_ip_allowed(&ip1));
    assert!(config.is_ip_allowed(&ip2));
}

#[test]
fn test_ip_whitelist_restricts_access() {
    let mut config = Config::default();

    let allowed_ip: IpAddr = "192.168.1.1".parse().expect("Failed to parse IP");
    let denied_ip: IpAddr = "10.0.0.1".parse().expect("Failed to parse IP");

    config.ip_whitelist.push(allowed_ip);

    assert!(config.is_ip_allowed(&allowed_ip));
    assert!(!config.is_ip_allowed(&denied_ip));
}

#[test]
fn test_ip_blacklist_denies_access() {
    let mut config = Config::default();

    let blacklisted_ip: IpAddr = "192.168.1.100".parse().expect("Failed to parse IP");
    let normal_ip: IpAddr = "192.168.1.1".parse().expect("Failed to parse IP");

    config.ip_blacklist.push(blacklisted_ip);

    assert!(!config.is_ip_allowed(&blacklisted_ip));
    assert!(config.is_ip_allowed(&normal_ip));
}

#[test]
fn test_ip_blacklist_overrides_whitelist() {
    let mut config = Config::default();

    let ip: IpAddr = "192.168.1.1".parse().expect("Failed to parse IP");

    config.ip_whitelist.push(ip);
    config.ip_blacklist.push(ip);

    // Blacklist should take precedence
    assert!(!config.is_ip_allowed(&ip));
}

#[test]
fn test_access_schedule_default() {
    let schedule = AccessSchedule::default();

    assert_eq!(schedule.allowed_days, vec![1, 2, 3, 4, 5]); // Mon-Fri
    assert_eq!(schedule.start_hour, 9);
    assert_eq!(schedule.end_hour, 17);
    assert_eq!(schedule.timezone, "UTC");
}

#[test]
fn test_access_schedule_validation_invalid_hour() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let mut config = Config::default();
    config.root_dir = temp_dir.path().to_path_buf();

    let mut user_config = UserConfig::default();
    user_config.access_schedule = Some(AccessSchedule {
        allowed_days: vec![1, 2, 3, 4, 5],
        start_hour: 25, // Invalid
        end_hour: 17,
        timezone: "UTC".to_string(),
    });

    config.users.insert("testuser".to_string(), user_config);

    assert!(config.validate().is_err());
}

#[test]
fn test_access_schedule_validation_invalid_day() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let mut config = Config::default();
    config.root_dir = temp_dir.path().to_path_buf();

    let mut user_config = UserConfig::default();
    user_config.access_schedule = Some(AccessSchedule {
        allowed_days: vec![1, 2, 7], // 7 is invalid
        start_hour: 9,
        end_hour: 17,
        timezone: "UTC".to_string(),
    });

    config.users.insert("testuser".to_string(), user_config);

    assert!(config.validate().is_err());
}

#[test]
fn test_user_config_read_only() {
    let mut config = Config::default();

    let mut user_config = UserConfig::default();
    user_config.read_only = true;

    config.users.insert("readonly_user".to_string(), user_config);

    // Read operations should be allowed
    assert!(config.is_operation_allowed("readonly_user", "read"));
    assert!(config.is_operation_allowed("readonly_user", "stat"));
    assert!(config.is_operation_allowed("readonly_user", "opendir"));
    assert!(config.is_operation_allowed("readonly_user", "readdir"));

    // Write operations should be denied
    assert!(!config.is_operation_allowed("readonly_user", "write"));
    assert!(!config.is_operation_allowed("readonly_user", "remove"));
    assert!(!config.is_operation_allowed("readonly_user", "mkdir"));
}

#[test]
fn test_user_config_allowed_operations() {
    let mut config = Config::default();

    let mut user_config = UserConfig::default();
    user_config.allowed_operations = Some(vec![
        "read".to_string(),
        "write".to_string(),
    ]);

    config.users.insert("limited_user".to_string(), user_config);

    // Allowed operations
    assert!(config.is_operation_allowed("limited_user", "read"));
    assert!(config.is_operation_allowed("limited_user", "write"));

    // Denied operations (not in allowed list)
    assert!(!config.is_operation_allowed("limited_user", "remove"));
    assert!(!config.is_operation_allowed("limited_user", "mkdir"));
}

#[test]
fn test_user_config_denied_operations() {
    let mut config = Config::default();

    let mut user_config = UserConfig::default();
    user_config.denied_operations = vec![
        "remove".to_string(),
        "rmdir".to_string(),
    ];

    config.users.insert("restricted_user".to_string(), user_config);

    // Allowed operations
    assert!(config.is_operation_allowed("restricted_user", "read"));
    assert!(config.is_operation_allowed("restricted_user", "write"));

    // Denied operations
    assert!(!config.is_operation_allowed("restricted_user", "remove"));
    assert!(!config.is_operation_allowed("restricted_user", "rmdir"));
}

#[test]
fn test_user_config_denied_overrides_allowed() {
    let mut config = Config::default();

    let mut user_config = UserConfig::default();
    user_config.allowed_operations = Some(vec![
        "read".to_string(),
        "write".to_string(),
        "remove".to_string(),
    ]);
    user_config.denied_operations = vec!["remove".to_string()];

    config.users.insert("conflicted_user".to_string(), user_config);

    // Denied should take precedence
    assert!(!config.is_operation_allowed("conflicted_user", "remove"));
    assert!(config.is_operation_allowed("conflicted_user", "read"));
    assert!(config.is_operation_allowed("conflicted_user", "write"));
}

#[test]
fn test_user_not_found_allows_all_operations() {
    let config = Config::default();

    // User doesn't exist, so all operations should be allowed
    assert!(config.is_operation_allowed("nonexistent_user", "read"));
    assert!(config.is_operation_allowed("nonexistent_user", "write"));
    assert!(config.is_operation_allowed("nonexistent_user", "remove"));
}

#[test]
fn test_multiple_users_configuration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let mut config = Config::default();
    config.root_dir = temp_dir.path().to_path_buf();

    // User 1: Read-only
    let mut user1 = UserConfig::default();
    user1.read_only = true;
    user1.bandwidth_limit = 1_000_000; // 1 MB/s

    // User 2: Limited operations
    let mut user2 = UserConfig::default();
    user2.allowed_operations = Some(vec!["read".to_string(), "write".to_string()]);
    user2.disk_quota = 10_000_000_000; // 10 GB

    // User 3: Time-restricted
    let mut user3 = UserConfig::default();
    user3.access_schedule = Some(AccessSchedule {
        allowed_days: vec![1, 2, 3, 4, 5], // Mon-Fri
        start_hour: 9,
        end_hour: 17,
        timezone: "UTC".to_string(),
    });

    config.users.insert("user1".to_string(), user1);
    config.users.insert("user2".to_string(), user2);
    config.users.insert("user3".to_string(), user3);

    assert!(config.validate().is_ok());
    assert_eq!(config.users.len(), 3);

    // Verify user1 config
    let u1 = config.get_user_config("user1").expect("User1 not found");
    assert!(u1.read_only);
    assert_eq!(u1.bandwidth_limit, 1_000_000);

    // Verify user2 config
    let u2 = config.get_user_config("user2").expect("User2 not found");
    assert_eq!(u2.disk_quota, 10_000_000_000);

    // Verify user3 config
    let u3 = config.get_user_config("user3").expect("User3 not found");
    assert!(u3.access_schedule.is_some());
}

#[test]
fn test_global_bandwidth_limit() {
    let mut config = Config::default();
    config.global_bandwidth_limit = 10_000_000; // 10 MB/s

    assert_eq!(config.global_bandwidth_limit, 10_000_000);
}

#[test]
fn test_config_with_all_features() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let mut config = Config::default();
    config.root_dir = temp_dir.path().to_path_buf();
    config.global_bandwidth_limit = 50_000_000; // 50 MB/s

    // IP whitelist
    config.ip_whitelist.push("192.168.1.0".parse().expect("Failed to parse IP"));
    config.ip_whitelist.push("10.0.0.0".parse().expect("Failed to parse IP"));

    // IP blacklist
    config.ip_blacklist.push("192.168.1.100".parse().expect("Failed to parse IP"));

    // User with all features
    let mut user_config = UserConfig::default();
    user_config.home_dir = Some(temp_dir.path().to_path_buf());
    user_config.bandwidth_limit = 5_000_000; // 5 MB/s
    user_config.disk_quota = 50_000_000_000; // 50 GB
    user_config.max_file_size = 1_000_000_000; // 1 GB
    user_config.max_connections = Some(3);
    user_config.access_schedule = Some(AccessSchedule::default());
    user_config.allowed_operations = Some(vec!["read".to_string(), "write".to_string()]);

    config.users.insert("power_user".to_string(), user_config);

    assert!(config.validate().is_ok());

    // Verify IP access
    let allowed: IpAddr = "192.168.1.1".parse().expect("Failed to parse IP");
    let blacklisted: IpAddr = "192.168.1.100".parse().expect("Failed to parse IP");
    let not_whitelisted: IpAddr = "172.16.0.1".parse().expect("Failed to parse IP");

    assert!(config.is_ip_allowed(&allowed));
    assert!(!config.is_ip_allowed(&blacklisted));
    assert!(!config.is_ip_allowed(&not_whitelisted));

    // Verify user config
    let user = config.get_user_config("power_user").expect("User not found");
    assert_eq!(user.bandwidth_limit, 5_000_000);
    assert_eq!(user.disk_quota, 50_000_000_000);
    assert_eq!(user.max_file_size, 1_000_000_000);
    assert_eq!(user.max_connections, Some(3));
}
