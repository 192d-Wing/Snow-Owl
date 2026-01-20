//! SFTP Server Binary
//!
//! NIST 800-53: AU-2 (Audit Events), AU-9 (Protection of Audit Information), AU-12 (Audit Generation)
//! STIG: V-222648 (Audit Records)
//! Implementation: Production-ready SFTP server with JSON logging for SIEM integration
//!
//! Run with: cargo run --bin snow-owl-sftp-server

use clap::Parser;
use snow_owl_sftp::{Config, LogFormat, Server};
use std::path::PathBuf;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

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

    /// Verbose logging (debug level)
    #[arg(short, long)]
    verbose: bool,

    /// Log format (json or text)
    #[arg(long)]
    log_format: Option<LogFormat>,

    /// Log file path
    #[arg(long)]
    log_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Load or create configuration
    let mut config = if let Some(config_path) = args.config {
        match Config::from_file(&config_path) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("Failed to load config: {}", e);
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

        if let Some(log_format) = args.log_format {
            config.logging.format = log_format;
        }

        if let Some(log_file) = args.log_file {
            config.logging.file = Some(log_file);
        }

        if args.verbose {
            config.logging.level = "debug".to_string();
        }

        config
    };

    // Initialize logging with JSON support for SIEM integration
    // NIST 800-53 AU-9: Protection of Audit Information
    // NIST 800-53 AU-12: Audit Generation
    // STIG V-222648: Audit records must be generated
    let _log_guard = if let Some(ref log_file) = config.logging.file {
        // Create log directory if it doesn't exist
        if let Some(parent) = log_file.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    eprintln!("Warning: Failed to create log directory: {}", e);
                    eprintln!("Falling back to stderr logging");
                    config.logging.file = None;
                }
            }
        }

        if config.logging.file.is_some() {
            let file_appender = tracing_appender::rolling::daily(
                log_file.parent().expect("log file must have parent directory"),
                log_file
                    .file_name()
                    .expect("log file must have filename")
                    .to_string_lossy()
                    .as_ref(),
            );
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

            match config.logging.format {
                LogFormat::Json => {
                    tracing_subscriber::fmt()
                        .json()
                        .with_env_filter(EnvFilter::new(config.logging.level.clone()))
                        .with_writer(non_blocking)
                        .with_current_span(true)
                        .with_span_list(true)
                        .init();
                }
                LogFormat::Text => {
                    tracing_subscriber::fmt()
                        .with_env_filter(EnvFilter::new(config.logging.level.clone()))
                        .with_writer(non_blocking)
                        .init();
                }
            }

            Some(guard)
        } else {
            None
        }
    } else {
        None
    };

    // If no file logging, log to stderr
    if _log_guard.is_none() {
        match config.logging.format {
            LogFormat::Json => {
                tracing_subscriber::fmt()
                    .json()
                    .with_env_filter(EnvFilter::new(config.logging.level.clone()))
                    .with_current_span(true)
                    .with_span_list(true)
                    .init();
            }
            LogFormat::Text => {
                tracing_subscriber::fmt()
                    .with_env_filter(EnvFilter::new(config.logging.level.clone()))
                    .init();
            }
        }
    }

    info!(
        event = "server_starting",
        version = env!("CARGO_PKG_VERSION"),
        "Starting Snow Owl SFTP Server"
    );

    // Ensure root directory exists
    if !config.root_dir.exists() {
        info!(
            event = "creating_root_directory",
            directory = ?config.root_dir,
            "Creating root directory"
        );
        if let Err(e) = std::fs::create_dir_all(&config.root_dir) {
            error!(
                event = "root_directory_creation_failed",
                directory = ?config.root_dir,
                error = %e,
                "Failed to create root directory"
            );
            std::process::exit(1);
        }
    }

    // Log configuration
    info!(
        event = "server_configuration",
        bind_address = %config.bind_address,
        port = config.port,
        root_dir = ?config.root_dir,
        max_connections = config.max_connections,
        max_connections_per_user = config.max_connections_per_user,
        max_auth_attempts = config.max_auth_attempts,
        log_format = ?config.logging.format,
        log_file = ?config.logging.file,
        audit_enabled = config.logging.audit_enabled,
        "SFTP Server Configuration"
    );

    // Validate configuration
    if let Err(e) = config.validate() {
        error!(
            event = "configuration_validation_failed",
            error = %e,
            "Configuration validation failed"
        );
        std::process::exit(1);
    }

    // Security configuration logging
    info!(
        event = "security_configuration",
        rate_limit_window_secs = config.rate_limit_window_secs,
        lockout_duration_secs = config.lockout_duration_secs,
        timeout_secs = config.timeout,
        "Security Configuration Active"
    );

    // Create and run server
    let server = match Server::new(config).await {
        Ok(s) => {
            info!(
                event = "server_created",
                "SFTP server created successfully"
            );
            s
        }
        Err(e) => {
            error!(
                event = "server_creation_failed",
                error = %e,
                "Failed to create server"
            );
            std::process::exit(1);
        }
    };

    info!(
        event = "server_running",
        "SFTP server is now running and accepting connections"
    );

    if let Err(e) = server.run().await {
        error!(
            event = "server_error",
            error = %e,
            "Server encountered an error"
        );
        std::process::exit(1);
    }

    info!(
        event = "server_shutdown",
        "SFTP server shutdown complete"
    );
}
