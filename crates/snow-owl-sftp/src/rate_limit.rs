//! Rate Limiting Module
//!
//! NIST 800-53: AC-7 (Unsuccessful Logon Attempts)
//! STIG: V-222578 - Implement replay-resistant authentication mechanisms
//! Implementation: Provides rate limiting for authentication attempts to prevent brute force attacks

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum authentication attempts per IP address
    pub max_attempts: u32,
    /// Time window for rate limiting (in seconds)
    pub window_secs: u64,
    /// Lockout duration after max attempts exceeded (in seconds)
    pub lockout_duration_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,           // 5 attempts
            window_secs: 300,          // 5 minutes
            lockout_duration_secs: 900, // 15 minutes lockout
        }
    }
}

/// Authentication attempt record
///
/// NIST 800-53: AC-7 (Unsuccessful Logon Attempts)
/// Implementation: Tracks authentication attempts for rate limiting
#[derive(Debug, Clone)]
struct AttemptRecord {
    /// Number of failed attempts
    failed_attempts: u32,
    /// Timestamp of first attempt in current window
    window_start: Instant,
    /// Timestamp when lockout ends (if locked out)
    lockout_until: Option<Instant>,
}

impl AttemptRecord {
    fn new() -> Self {
        Self {
            failed_attempts: 0,
            window_start: Instant::now(),
            lockout_until: None,
        }
    }
}

/// Rate limiter for authentication attempts
///
/// NIST 800-53: AC-7 (Unsuccessful Logon Attempts)
/// STIG: V-222578 - Replay-resistant authentication
/// Implementation: Tracks and limits authentication attempts per IP address
pub struct RateLimiter {
    config: RateLimitConfig,
    attempts: Arc<Mutex<HashMap<IpAddr, AttemptRecord>>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    ///
    /// * `config` - Rate limiting configuration
    ///
    /// # Returns
    ///
    /// A new `RateLimiter` instance
    ///
    /// # NIST 800-53: AC-7 (Unsuccessful Logon Attempts)
    /// # Implementation: Initializes rate limiting system
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            attempts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if an IP address is allowed to attempt authentication
    ///
    /// # Arguments
    ///
    /// * `ip` - IP address to check
    ///
    /// # Returns
    ///
    /// `true` if authentication attempt is allowed, `false` if rate limited
    ///
    /// # NIST 800-53: AC-7 (Unsuccessful Logon Attempts)
    /// # Implementation: Checks if IP has exceeded attempt limit or is locked out
    pub async fn check_allowed(&self, ip: IpAddr) -> bool {
        let mut attempts = self.attempts.lock().await;

        // Get or create attempt record
        let record = attempts.entry(ip).or_insert_with(AttemptRecord::new);

        // Check if currently locked out
        if let Some(lockout_until) = record.lockout_until {
            if Instant::now() < lockout_until {
                warn!(
                    "IP {} is locked out until {:?}",
                    ip,
                    lockout_until.duration_since(Instant::now())
                );
                return false;
            } else {
                // Lockout expired, reset
                debug!("Lockout expired for IP {}", ip);
                record.lockout_until = None;
                record.failed_attempts = 0;
                record.window_start = Instant::now();
            }
        }

        // Check if we need to reset the window
        let window_duration = Duration::from_secs(self.config.window_secs);
        if Instant::now().duration_since(record.window_start) > window_duration {
            debug!("Resetting rate limit window for IP {}", ip);
            record.failed_attempts = 0;
            record.window_start = Instant::now();
        }

        // Check if within rate limit
        let allowed = record.failed_attempts < self.config.max_attempts;

        if !allowed {
            warn!(
                "IP {} exceeded rate limit ({}/{} attempts)",
                ip, record.failed_attempts, self.config.max_attempts
            );
        }

        allowed
    }

    /// Record a failed authentication attempt
    ///
    /// # Arguments
    ///
    /// * `ip` - IP address of the failed attempt
    ///
    /// # NIST 800-53: AC-7 (Unsuccessful Logon Attempts)
    /// # STIG: V-222578
    /// # Implementation: Records failed attempt and enforces lockout if limit exceeded
    pub async fn record_failure(&self, ip: IpAddr) {
        let mut attempts = self.attempts.lock().await;

        let record = attempts.entry(ip).or_insert_with(AttemptRecord::new);

        // Increment failure count
        record.failed_attempts += 1;

        warn!(
            "Failed authentication attempt from IP {} ({}/{})",
            ip, record.failed_attempts, self.config.max_attempts
        );

        // Check if we need to lock out
        if record.failed_attempts >= self.config.max_attempts {
            let lockout_duration = Duration::from_secs(self.config.lockout_duration_secs);
            record.lockout_until = Some(Instant::now() + lockout_duration);

            warn!(
                "IP {} locked out for {} seconds due to {} failed attempts",
                ip, self.config.lockout_duration_secs, record.failed_attempts
            );
        }
    }

    /// Record a successful authentication
    ///
    /// # Arguments
    ///
    /// * `ip` - IP address of the successful attempt
    ///
    /// # NIST 800-53: AC-7 (Unsuccessful Logon Attempts)
    /// # Implementation: Clears failure count on successful authentication
    pub async fn record_success(&self, ip: IpAddr) {
        let mut attempts = self.attempts.lock().await;

        if let Some(record) = attempts.get_mut(&ip) {
            if record.failed_attempts > 0 {
                debug!(
                    "Clearing {} failed attempts for IP {} after successful auth",
                    record.failed_attempts, ip
                );
                record.failed_attempts = 0;
                record.lockout_until = None;
            }
        }
    }

    /// Clean up old entries to prevent memory growth
    ///
    /// # NIST 800-53: AC-7 (Unsuccessful Logon Attempts)
    /// # Implementation: Removes expired records to manage memory
    pub async fn cleanup_expired(&self) {
        let mut attempts = self.attempts.lock().await;

        let window_duration = Duration::from_secs(self.config.window_secs);
        let now = Instant::now();

        // Remove entries where:
        // 1. Window has expired and no lockout
        // 2. Lockout has expired
        attempts.retain(|ip, record| {
            // If locked out, keep until lockout expires
            if let Some(lockout_until) = record.lockout_until {
                if now < lockout_until {
                    return true; // Keep locked out entries
                }
            }

            // Check if window has expired
            let keep = now.duration_since(record.window_start) <= window_duration
                || record.failed_attempts > 0;

            if !keep {
                debug!("Cleaning up expired rate limit entry for IP {}", ip);
            }

            keep
        });
    }

    /// Get current statistics for monitoring
    ///
    /// # Returns
    ///
    /// Tuple of (total IPs tracked, locked out IPs)
    pub async fn get_stats(&self) -> (usize, usize) {
        let attempts = self.attempts.lock().await;
        let total = attempts.len();
        let locked_out = attempts
            .values()
            .filter(|r| {
                r.lockout_until
                    .map(|until| Instant::now() < until)
                    .unwrap_or(false)
            })
            .count();

        (total, locked_out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_rate_limiter_allows_initial_attempts() {
        let config = RateLimitConfig {
            max_attempts: 3,
            window_secs: 60,
            lockout_duration_secs: 120,
        };

        let limiter = RateLimiter::new(config);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // First attempt should be allowed
        assert!(limiter.check_allowed(ip).await);
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_after_max_attempts() {
        let config = RateLimitConfig {
            max_attempts: 3,
            window_secs: 60,
            lockout_duration_secs: 120,
        };

        let limiter = RateLimiter::new(config);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // Record failures up to limit
        for _ in 0..3 {
            limiter.record_failure(ip).await;
        }

        // Next attempt should be blocked
        assert!(!limiter.check_allowed(ip).await);
    }

    #[tokio::test]
    async fn test_rate_limiter_resets_on_success() {
        let config = RateLimitConfig {
            max_attempts: 3,
            window_secs: 60,
            lockout_duration_secs: 120,
        };

        let limiter = RateLimiter::new(config);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // Record some failures
        limiter.record_failure(ip).await;
        limiter.record_failure(ip).await;

        // Record success
        limiter.record_success(ip).await;

        // Should be allowed again
        assert!(limiter.check_allowed(ip).await);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let config = RateLimitConfig::default();
        let limiter = RateLimiter::new(config);

        let ip1 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2));

        limiter.record_failure(ip1).await;
        limiter.record_failure(ip2).await;

        let (total, _locked) = limiter.get_stats().await;
        assert_eq!(total, 2);
    }
}
