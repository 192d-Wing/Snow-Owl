/// Authentication and User Management Commands
///
/// NIST Controls:
/// - AC-2: Account Management
/// - IA-5: Authenticator Management
/// - AU-2: Audit Events

use anyhow::{Context, Result};
use chrono::Utc;
use snow_owl_core::{User, ApiKey, UserRole};
use snow_owl_db::Database;
use snow_owl_http::auth::{generate_api_key, hash_api_key};
use uuid::Uuid;
use std::path::Path;

use crate::config;

/// User management subcommands
#[derive(clap::Subcommand)]
pub enum UserCommands {
    /// Create a new user
    Create {
        /// Username
        username: String,

        /// User role (admin, operator, readonly)
        #[arg(short, long)]
        role: String,
    },

    /// List all users
    List,

    /// Show user details
    Info {
        /// Username
        username: String,
    },
}

/// API key management subcommands
#[derive(clap::Subcommand)]
pub enum ApiKeyCommands {
    /// Generate a new API key for a user
    Create {
        /// Username
        username: String,

        /// Key name/description
        #[arg(short, long)]
        name: String,

        /// Expiration in days (optional)
        #[arg(short, long)]
        expires: Option<i64>,
    },

    /// List API keys for a user
    List {
        /// Username
        username: String,
    },

    /// Revoke an API key
    Revoke {
        /// API key ID
        key_id: String,
    },
}

/// Handle user management commands
///
/// NIST Controls:
/// - AC-2: Account Management
pub async fn handle_user(config_path: &Path, cmd: UserCommands) -> Result<()> {
    let config = config::load_config(config_path)
        .await
        .context("Failed to load configuration")?;

    let db = Database::new(&config.database_url)
        .await
        .context("Failed to connect to database")?;

    match cmd {
        UserCommands::Create { username, role } => {
            // NIST AC-2: Parse and validate role
            let user_role = match role.to_lowercase().as_str() {
                "admin" => UserRole::Admin,
                "operator" => UserRole::Operator,
                "readonly" => UserRole::ReadOnly,
                _ => {
                    anyhow::bail!("Invalid role: {}. Must be one of: admin, operator, readonly", role);
                }
            };

            // NIST AC-2: Create user account
            let user = User {
                id: Uuid::new_v4(),
                username: username.clone(),
                role: user_role,
                created_at: Utc::now(),
                last_login: None,
            };

            db.create_user(&user).await?;

            println!("✓ User created successfully");
            println!("  ID: {}", user.id);
            println!("  Username: {}", user.username);
            println!("  Role: {}", user.role);
            println!("\nNext steps:");
            println!("  Generate an API key with:");
            println!("  snow-owl api-key create {} --name \"My API Key\"", username);
        }

        UserCommands::List => {
            // NIST AC-2: List all user accounts
            let users = db.list_users().await?;

            if users.is_empty() {
                println!("No users found.");
                println!("\nCreate the first admin user with:");
                println!("  snow-owl user create admin --role admin");
            } else {
                println!("Users:");
                println!();
                for user in users {
                    println!("  {} ({})", user.username, user.role);
                    println!("    ID: {}", user.id);
                    println!("    Created: {}", user.created_at.format("%Y-%m-%d %H:%M:%S"));
                    if let Some(last_login) = user.last_login {
                        println!("    Last login: {}", last_login.format("%Y-%m-%d %H:%M:%S"));
                    }
                    println!();
                }
            }
        }

        UserCommands::Info { username } => {
            let user = db.get_user_by_username(&username).await?
                .context(format!("User not found: {}", username))?;

            println!("User: {}", user.username);
            println!("  ID: {}", user.id);
            println!("  Role: {}", user.role);
            println!("  Created: {}", user.created_at.format("%Y-%m-%d %H:%M:%S"));
            if let Some(last_login) = user.last_login {
                println!("  Last login: {}", last_login.format("%Y-%m-%d %H:%M:%S"));
            }

            // Show API keys
            let keys = db.list_user_api_keys(user.id).await?;
            println!("\nAPI Keys: {}", keys.len());
            for key in keys {
                println!("  {} ({})", key.name, key.id);
                println!("    Created: {}", key.created_at.format("%Y-%m-%d %H:%M:%S"));
                if let Some(expires) = key.expires_at {
                    println!("    Expires: {}", expires.format("%Y-%m-%d %H:%M:%S"));
                }
                if let Some(last_used) = key.last_used {
                    println!("    Last used: {}", last_used.format("%Y-%m-%d %H:%M:%S"));
                }
                println!();
            }
        }
    }

    Ok(())
}

/// Handle API key management commands
///
/// NIST Controls:
/// - IA-5: Authenticator Management
/// - SC-12: Cryptographic Key Establishment
pub async fn handle_api_key(config_path: &Path, cmd: ApiKeyCommands) -> Result<()> {
    let config = config::load_config(config_path)
        .await
        .context("Failed to load configuration")?;

    let db = Database::new(&config.database_url)
        .await
        .context("Failed to connect to database")?;

    match cmd {
        ApiKeyCommands::Create { username, name, expires } => {
            // NIST IA-2: Lookup user
            let user = db.get_user_by_username(&username).await?
                .context(format!("User not found: {}", username))?;

            // NIST SC-12: Generate cryptographically secure API key
            let key = generate_api_key();

            // NIST SC-13: Hash the key before storage
            let key_hash = hash_api_key(&key);

            // NIST IA-5: Calculate expiration if specified
            let expires_at = expires.map(|days| {
                Utc::now() + chrono::Duration::days(days)
            });

            // NIST IA-5: Create API key record
            let api_key = ApiKey {
                id: Uuid::new_v4(),
                user_id: user.id,
                name: name.clone(),
                key_hash,
                created_at: Utc::now(),
                expires_at,
                last_used: None,
            };

            db.create_api_key(&api_key).await?;

            println!("✓ API key created successfully");
            println!();
            println!("  User: {}", user.username);
            println!("  Name: {}", name);
            println!("  Key ID: {}", api_key.id);
            if let Some(exp) = expires_at {
                println!("  Expires: {}", exp.format("%Y-%m-%d %H:%M:%S"));
            }
            println!();
            println!("  API Key: {}", key);
            println!();
            println!("⚠ IMPORTANT: Store this API key securely!");
            println!("  This is the only time you will see the full key.");
            println!("  The key is stored as a hash and cannot be recovered.");
            println!();
            println!("Usage:");
            println!("  curl -H \"Authorization: Bearer {}\" \\", key);
            println!("    http://localhost:8080/api/machines");
        }

        ApiKeyCommands::List { username } => {
            // NIST AC-2: Lookup user
            let user = db.get_user_by_username(&username).await?
                .context(format!("User not found: {}", username))?;

            // NIST IA-5: List all API keys for user
            let keys = db.list_user_api_keys(user.id).await?;

            println!("API Keys for user '{}':", username);
            println!();

            if keys.is_empty() {
                println!("No API keys found.");
                println!("\nCreate one with:");
                println!("  snow-owl api-key create {} --name \"My Key\"", username);
            } else {
                for key in keys {
                    println!("  {} ({})", key.name, key.id);
                    println!("    Created: {}", key.created_at.format("%Y-%m-%d %H:%M:%S"));
                    if let Some(expires) = key.expires_at {
                        let now = Utc::now();
                        if expires < now {
                            println!("    Status: ⚠ EXPIRED");
                        } else {
                            println!("    Expires: {}", expires.format("%Y-%m-%d %H:%M:%S"));
                        }
                    } else {
                        println!("    Expires: Never");
                    }
                    if let Some(last_used) = key.last_used {
                        println!("    Last used: {}", last_used.format("%Y-%m-%d %H:%M:%S"));
                    }
                    println!();
                }
            }
        }

        ApiKeyCommands::Revoke { key_id } => {
            // NIST IA-5: Parse key ID
            let id = Uuid::parse_str(&key_id)
                .context("Invalid API key ID format")?;

            // NIST AC-2(4): Revoke API key
            db.revoke_api_key(id).await?;

            println!("✓ API key revoked successfully");
            println!("  The key can no longer be used for authentication.");
        }
    }

    Ok(())
}
