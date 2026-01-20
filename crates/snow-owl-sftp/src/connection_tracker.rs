//! Connection Tracking Module
//!
//! NIST 800-53: AC-12 (Session Termination), AC-10 (Concurrent Session Control)
//! STIG: V-222601 - The application must terminate sessions after organization-defined conditions
//! Implementation: Tracks and limits concurrent connections per user

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// Configuration for connection tracking
#[derive(Debug, Clone)]
pub struct ConnectionTrackerConfig {
    /// Maximum concurrent connections per user
    pub max_connections_per_user: usize,
}

impl Default for ConnectionTrackerConfig {
    fn default() -> Self {
        Self {
            max_connections_per_user: 10,
        }
    }
}

/// Tracks active connections per user
///
/// NIST 800-53: AC-10 (Concurrent Session Control), AC-12 (Session Termination)
/// STIG: V-222601 - Session termination
/// Implementation: Enforces maximum concurrent connections per user
pub struct ConnectionTracker {
    config: ConnectionTrackerConfig,
    /// Maps username to list of connection IDs
    connections: Arc<Mutex<HashMap<String, Vec<usize>>>>,
    next_connection_id: Arc<Mutex<usize>>,
}

impl ConnectionTracker {
    /// Create a new connection tracker
    ///
    /// # Arguments
    ///
    /// * `config` - Connection tracking configuration
    ///
    /// # Returns
    ///
    /// A new `ConnectionTracker` instance
    ///
    /// # NIST 800-53: AC-10 (Concurrent Session Control)
    /// # Implementation: Initializes connection tracking system
    pub fn new(config: ConnectionTrackerConfig) -> Self {
        Self {
            config,
            connections: Arc::new(Mutex::new(HashMap::new())),
            next_connection_id: Arc::new(Mutex::new(0)),
        }
    }

    /// Check if a user can establish a new connection
    ///
    /// # Arguments
    ///
    /// * `username` - Username attempting to connect
    ///
    /// # Returns
    ///
    /// `true` if connection is allowed, `false` if limit exceeded
    ///
    /// # NIST 800-53: AC-10 (Concurrent Session Control)
    /// # Implementation: Checks if user has exceeded connection limit
    pub async fn can_connect(&self, username: &str) -> bool {
        let connections = self.connections.lock().await;

        let current_count = connections
            .get(username)
            .map(|conns| conns.len())
            .unwrap_or(0);

        let allowed = current_count < self.config.max_connections_per_user;

        if !allowed {
            warn!(
                "User '{}' exceeded max connections ({}/{})",
                username, current_count, self.config.max_connections_per_user
            );
        }

        allowed
    }

    /// Register a new connection for a user
    ///
    /// # Arguments
    ///
    /// * `username` - Username of the connecting user
    ///
    /// # Returns
    ///
    /// Connection ID if successful, `None` if limit exceeded
    ///
    /// # NIST 800-53: AC-10 (Concurrent Session Control)
    /// # STIG: V-222601
    /// # Implementation: Tracks new connection and enforces limit
    pub async fn register_connection(&self, username: String) -> Option<usize> {
        let mut connections = self.connections.lock().await;

        // Check limit before registering
        let current_count = connections
            .get(&username)
            .map(|conns| conns.len())
            .unwrap_or(0);

        if current_count >= self.config.max_connections_per_user {
            warn!(
                "Rejecting connection for user '{}' - max connections ({}) exceeded",
                username, self.config.max_connections_per_user
            );
            return None;
        }

        // Allocate connection ID
        let mut next_id = self.next_connection_id.lock().await;
        let connection_id = *next_id;
        *next_id = next_id.wrapping_add(1);
        drop(next_id);

        // Register connection
        connections
            .entry(username.clone())
            .or_insert_with(Vec::new)
            .push(connection_id);

        info!(
            "Registered connection {} for user '{}' ({}/{})",
            connection_id,
            username,
            current_count + 1,
            self.config.max_connections_per_user
        );

        Some(connection_id)
    }

    /// Unregister a connection
    ///
    /// # Arguments
    ///
    /// * `username` - Username of the disconnecting user
    /// * `connection_id` - Connection ID to remove
    ///
    /// # NIST 800-53: AC-12 (Session Termination)
    /// # Implementation: Removes connection from tracking
    pub async fn unregister_connection(&self, username: &str, connection_id: usize) {
        let mut connections = self.connections.lock().await;

        if let Some(user_conns) = connections.get_mut(username) {
            user_conns.retain(|&id| id != connection_id);

            let remaining = user_conns.len();

            if remaining == 0 {
                // Remove user entry if no connections remain
                connections.remove(username);
                debug!("User '{}' has no remaining connections", username);
            } else {
                info!(
                    "Unregistered connection {} for user '{}' ({} remaining)",
                    connection_id, username, remaining
                );
            }
        }
    }

    /// Get current connection count for a user
    ///
    /// # Arguments
    ///
    /// * `username` - Username to check
    ///
    /// # Returns
    ///
    /// Number of active connections for the user
    pub async fn get_connection_count(&self, username: &str) -> usize {
        let connections = self.connections.lock().await;
        connections
            .get(username)
            .map(|conns| conns.len())
            .unwrap_or(0)
    }

    /// Get overall statistics
    ///
    /// # Returns
    ///
    /// Tuple of (total active users, total connections)
    pub async fn get_stats(&self) -> (usize, usize) {
        let connections = self.connections.lock().await;
        let total_users = connections.len();
        let total_connections: usize = connections.values().map(|conns| conns.len()).sum();

        (total_users, total_connections)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_limit_enforcement() {
        let config = ConnectionTrackerConfig {
            max_connections_per_user: 2,
        };

        let tracker = ConnectionTracker::new(config);

        // First connection should succeed
        assert!(tracker.can_connect("alice").await);
        let conn1 = tracker.register_connection("alice".to_string()).await;
        assert!(conn1.is_some());

        // Second connection should succeed
        assert!(tracker.can_connect("alice").await);
        let conn2 = tracker.register_connection("alice".to_string()).await;
        assert!(conn2.is_some());

        // Third connection should fail (limit = 2)
        assert!(!tracker.can_connect("alice").await);
        let conn3 = tracker.register_connection("alice".to_string()).await;
        assert!(conn3.is_none());

        // After unregistering one, should allow new connection
        tracker
            .unregister_connection("alice", conn1.unwrap())
            .await;
        assert!(tracker.can_connect("alice").await);
        let conn4 = tracker.register_connection("alice".to_string()).await;
        assert!(conn4.is_some());
    }

    #[tokio::test]
    async fn test_multiple_users() {
        let config = ConnectionTrackerConfig {
            max_connections_per_user: 2,
        };

        let tracker = ConnectionTracker::new(config);

        // Alice can connect twice
        let alice1 = tracker.register_connection("alice".to_string()).await;
        let alice2 = tracker.register_connection("alice".to_string()).await;
        assert!(alice1.is_some());
        assert!(alice2.is_some());

        // Bob can also connect twice (separate limit)
        let bob1 = tracker.register_connection("bob".to_string()).await;
        let bob2 = tracker.register_connection("bob".to_string()).await;
        assert!(bob1.is_some());
        assert!(bob2.is_some());

        // Both at limit
        assert!(!tracker.can_connect("alice").await);
        assert!(!tracker.can_connect("bob").await);

        let (users, conns) = tracker.get_stats().await;
        assert_eq!(users, 2);
        assert_eq!(conns, 4);
    }

    #[tokio::test]
    async fn test_cleanup_on_disconnect() {
        let config = ConnectionTrackerConfig {
            max_connections_per_user: 3,
        };

        let tracker = ConnectionTracker::new(config);

        let conn1 = tracker.register_connection("alice".to_string()).await.unwrap();
        let conn2 = tracker.register_connection("alice".to_string()).await.unwrap();

        assert_eq!(tracker.get_connection_count("alice").await, 2);

        // Unregister all connections
        tracker.unregister_connection("alice", conn1).await;
        assert_eq!(tracker.get_connection_count("alice").await, 1);

        tracker.unregister_connection("alice", conn2).await;
        assert_eq!(tracker.get_connection_count("alice").await, 0);

        // User entry should be removed
        let (users, _) = tracker.get_stats().await;
        assert_eq!(users, 0);
    }
}
