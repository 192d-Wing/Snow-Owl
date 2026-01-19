/// Authentication and Authorization Module
///
/// NIST Controls:
/// - AC-2: Account Management
/// - AC-3: Access Enforcement
/// - IA-2: Identification and Authentication
/// - IA-5: Authenticator Management
/// - AU-2: Audit Events
/// - AU-3: Content of Audit Records
use axum::{
    extract::{Request, State},
    http::{StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};
use snow_owl_core::{SnowOwlError, User, UserRole};
use snow_owl_db::Database;
use std::sync::Arc;
use tracing::{info, warn};

/// Authentication state passed through request extensions
///
/// NIST Controls:
/// - IA-2: Identification and Authentication
/// - AC-3: Access Enforcement (user context for authorization)
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user: User,
}

/// Generate API key
///
/// NIST Controls:
/// - SC-12: Cryptographic Key Establishment and Management
/// - SC-13: Cryptographic Protection
pub fn generate_api_key() -> String {
    use uuid::Uuid;
    // Generate a cryptographically secure random API key
    // Format: so_<uuid> (snow-owl prefix)
    format!("so_{}", Uuid::new_v4())
}

/// Hash API key for storage
///
/// NIST Controls:
/// - SC-13: Cryptographic Protection (SHA-256 hashing)
/// - IA-5(1): Password-based Authentication (secure credential storage)
pub fn hash_api_key(key: &str) -> String {
    // NIST SC-13: Use SHA-256 for one-way hashing
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let result = hasher.finalize();
    // Convert to hex string
    hex::encode(result)
}

/// Authentication middleware
///
/// NIST Controls:
/// - IA-2: Identification and Authentication
/// - AC-3: Access Enforcement
/// - AU-3: Content of Audit Records (log auth attempts)
pub async fn auth_middleware(
    State(db): State<Arc<Database>>,
    mut request: Request,
    next: Next,
) -> std::result::Result<Response, StatusCode> {
    // NIST IA-2: Extract and validate Authorization header
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(auth_value) = auth_header {
        // NIST IA-2: Support Bearer token format
        if let Some(token) = auth_value.strip_prefix("Bearer ") {
            // NIST SC-13: Hash the provided key to compare with stored hash
            let key_hash = hash_api_key(token);

            // NIST IA-2: Validate API key against database
            match db.validate_api_key(&key_hash).await {
                Ok(Some((user, api_key))) => {
                    // NIST AU-3: Log successful authentication
                    info!(
                        "Authenticated user: {} (role: {}) via API key: {}",
                        user.username, user.role, api_key.name
                    );

                    // NIST AU-3: Update last used timestamp
                    let _ = db.update_api_key_last_used(api_key.id).await;
                    let _ = db.update_user_last_login(user.id).await;

                    // NIST AC-3: Store authenticated user in request extensions
                    request.extensions_mut().insert(AuthUser { user });

                    // Continue to next middleware/handler
                    return Ok(next.run(request).await);
                }
                Ok(None) => {
                    // NIST AU-3: Log failed authentication attempt
                    warn!("Invalid or expired API key");
                }
                Err(e) => {
                    // NIST AU-3: Log authentication errors
                    warn!("Authentication error: {}", e);
                }
            }
        }
    }

    // NIST AC-3: Deny access if authentication fails
    // NIST AU-3: Log unauthorized access attempt
    warn!("Unauthorized access attempt");
    Err(StatusCode::UNAUTHORIZED)
}

/// Optional authentication middleware (allows unauthenticated requests)
///
/// Useful for endpoints that have public access but may have enhanced
/// functionality when authenticated.
pub async fn optional_auth_middleware(
    State(db): State<Arc<Database>>,
    mut request: Request,
    next: Next,
) -> Response {
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(auth_value) = auth_header {
        if let Some(token) = auth_value.strip_prefix("Bearer ") {
            let key_hash = hash_api_key(token);

            if let Ok(Some((user, _))) = db.validate_api_key(&key_hash).await {
                request.extensions_mut().insert(AuthUser { user });
            }
        }
    }

    next.run(request).await
}

/// Check if user has required role
///
/// NIST Controls:
/// - AC-3: Access Enforcement
/// - AC-6: Least Privilege
pub fn check_role(user: &User, required_role: UserRole) -> bool {
    match required_role {
        // NIST AC-6(10): Admin can do everything
        UserRole::Admin => user.role == UserRole::Admin,
        // NIST AC-6(5): Operator can do operator and readonly
        UserRole::Operator => matches!(user.role, UserRole::Admin | UserRole::Operator),
        // NIST AC-6: Everyone can do readonly
        UserRole::ReadOnly => true,
    }
}

/// Extract authenticated user from request
///
/// NIST Controls:
/// - IA-2: Identification and Authentication
pub fn get_auth_user(request: &Request) -> std::result::Result<User, SnowOwlError> {
    request
        .extensions()
        .get::<AuthUser>()
        .map(|auth| auth.user.clone())
        .ok_or_else(|| SnowOwlError::Http("Unauthorized".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_generation() {
        let key1 = generate_api_key();
        let key2 = generate_api_key();

        assert!(key1.starts_with("so_"));
        assert!(key2.starts_with("so_"));
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_api_key_hashing() {
        let key = "so_test-key-123";
        let hash1 = hash_api_key(key);
        let hash2 = hash_api_key(key);

        // Same key should produce same hash
        assert_eq!(hash1, hash2);

        // Hash should be different from key
        assert_ne!(hash1, key);

        // Hash should be hex string (64 chars for SHA-256)
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_role_hierarchy() {
        use chrono::Utc;
        use uuid::Uuid;

        let admin = User {
            id: Uuid::new_v4(),
            username: "admin".to_string(),
            role: UserRole::Admin,
            created_at: Utc::now(),
            last_login: None,
        };

        let operator = User {
            id: Uuid::new_v4(),
            username: "operator".to_string(),
            role: UserRole::Operator,
            created_at: Utc::now(),
            last_login: None,
        };

        let readonly = User {
            id: Uuid::new_v4(),
            username: "readonly".to_string(),
            role: UserRole::ReadOnly,
            created_at: Utc::now(),
            last_login: None,
        };

        // Admin can do everything
        assert!(check_role(&admin, UserRole::Admin));
        assert!(check_role(&admin, UserRole::Operator));
        assert!(check_role(&admin, UserRole::ReadOnly));

        // Operator can do operator and readonly
        assert!(!check_role(&operator, UserRole::Admin));
        assert!(check_role(&operator, UserRole::Operator));
        assert!(check_role(&operator, UserRole::ReadOnly));

        // ReadOnly can only do readonly
        assert!(!check_role(&readonly, UserRole::Admin));
        assert!(!check_role(&readonly, UserRole::Operator));
        assert!(check_role(&readonly, UserRole::ReadOnly));
    }
}
