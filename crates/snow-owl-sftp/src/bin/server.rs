//! SFTP Server Binary
//!
//! Run with: cargo run --bin snow-owl-sftp-server

use clap::Parser;
use snow_owl_sftp::{Config, Server};
use std::path::PathBuf;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,

    /// Bind address
    #[arg(short, long, default_value = "0.0.0.0")]
    bind: String,

    /// Port to listen on
    #[arg(short, long, default_value = "2222")]
    port: u16,

    /// Root directory for SFTP operations
    #[arg(short, long)]
    root: Option<PathBuf>,

    /// Host key path
    #[arg(long)]
    host_key: Option<PathBuf>,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Initialize tracing
    let filter = if args.verbose {
        "debug,russh=info"
    } else {
        "info,russh=warn"
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| filter.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load or create configuration
    let config = if let Some(config_path) = args.config {
        match Config::from_file(&config_path) {
            Ok(cfg) => cfg,
            Err(e) => {
                error!("Failed to load config: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        let mut config = Config::default();
        config.bind_address = args.bind;
        config.port = args.port;
        config.verbose = args.verbose;

        if let Some(root) = args.root {
            config.root_dir = root;
        }

        if let Some(host_key) = args.host_key {
            config.host_key_path = host_key;
        }

        config
    };

    // Ensure root directory exists
    if !config.root_dir.exists() {
        info!("Creating root directory: {:?}", config.root_dir);
        if let Err(e) = std::fs::create_dir_all(&config.root_dir) {
            error!("Failed to create root directory: {}", e);
            std::process::exit(1);
        }
    }

    info!("SFTP Server Configuration:");
    info!("  Bind Address: {}", config.bind_address);
    info!("  Port: {}", config.port);
    info!("  Root Directory: {:?}", config.root_dir);
    info!("  Max Connections: {}", config.max_connections);

    // Create and run server
    let server = match Server::new(config).await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to create server: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = server.run().await {
        error!("Server error: {}", e);
        std::process::exit(1);
    }
}
