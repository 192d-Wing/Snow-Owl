//! # Snow Owl SFTP
//!
//! RFC-compliant SFTP (SSH File Transfer Protocol) implementation.
//!
//! This crate implements the SFTP protocol as defined in:
//! - RFC 4251: SSH Protocol Architecture
//! - RFC 4252: SSH Authentication Protocol
//! - RFC 4253: SSH Transport Layer Protocol
//! - RFC 4254: SSH Connection Protocol
//! - draft-ietf-secsh-filexfer-02: SSH File Transfer Protocol
//!
//! ## Features
//!
//! - Full SFTP protocol support
//! - Async/await with Tokio
//! - SSH key-based authentication
//! - File operations (read, write, delete, rename)
//! - Directory operations (list, create, remove)
//! - File attribute management

pub mod auth;
pub mod config;
pub mod error;
pub mod protocol;
pub mod server;
pub mod client;

pub use auth::AuthorizedKeys;
pub use config::Config;
pub use error::{Error, Result};
pub use server::Server;
pub use client::Client;
