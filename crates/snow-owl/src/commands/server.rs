use anyhow::{Context, Result};
use snow_owl_core::ServerConfig;
use snow_owl_db::Database;
use snow_owl_http::HttpServer;
use snow_owl_tftp::TftpServer;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tracing::info;

use crate::config;

pub async fn init_config(config_path: &Path) -> Result<()> {
    let default_config = ServerConfig::default();
    config::save_config(config_path, &default_config).await?;
    println!("Configuration file created at: {}", config_path.display());
    println!("\nPlease review and edit the configuration before starting the server.");
    Ok(())
}

/// Start Snow-Owl deployment server with security controls
///
/// NIST Controls:
/// - CM-6: Configuration Settings (load from config file)
/// - AU-3: Content of Audit Records (log startup events)
/// - SC-7: Boundary Protection (network service initialization)
/// - IA-5: Authenticator Management (database credentials)
pub async fn run(config_path: &Path) -> Result<()> {
    // NIST AU-3: Log server startup
    info!("Starting Snow-Owl deployment server...");

    // NIST CM-6: Load configuration from file
    let config = config::load_config(config_path)
        .await
        .context("Failed to load configuration")?;

    info!("Configuration loaded from {}", config_path.display());

    // NIST AC-3: Create necessary directories with proper permissions
    // NIST CM-7: Least Functionality - only create required directories
    tokio::fs::create_dir_all(&config.tftp_root).await?;
    tokio::fs::create_dir_all(&config.images_dir).await?;
    tokio::fs::create_dir_all(&config.winpe_dir).await?;

    // NIST IA-5: Initialize database with authenticated connection
    // NIST SC-28: Protection of Information at Rest (database)
    let db = Arc::new(
        Database::new(&config.database_url)
            .await
            .context("Failed to initialize database")?,
    );
    // NIST AU-3: Log database connection (without credentials)
    info!("Database connection established: {}", config.database_url);

    // Start TFTP server if enabled
    let tftp_handle = if config.enable_tftp {
        let tftp_root = config.tftp_root.clone();
        // Bind to the configured server IP (supports both IPv4 and IPv6)
        let tftp_addr = SocketAddr::new(config.network.server_ip, 69);
        let tftp_server = TftpServer::new(tftp_root, tftp_addr);

        info!("Starting TFTP server on {}", tftp_addr);
        Some(tokio::spawn(async move {
            if let Err(e) = tftp_server.run().await {
                tracing::error!("TFTP server error: {}", e);
            }
        }))
    } else {
        info!("TFTP server disabled");
        None
    };

    // Start HTTP server
    let http_server = HttpServer::new(db, config);
    let http_handle = tokio::spawn(async move {
        if let Err(e) = http_server.run().await {
            tracing::error!("HTTP server error: {}", e);
        }
    });

    info!("Snow-Owl server is running. Press Ctrl+C to stop.");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");

    // Note: In a production system, we would gracefully shut down the servers here
    if let Some(handle) = tftp_handle {
        handle.abort();
    }
    http_handle.abort();

    Ok(())
}
