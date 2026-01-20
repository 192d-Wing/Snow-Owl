//! SFTP Client Binary
//!
//! Run with: cargo run --bin snow-owl-sftp-client

use clap::{Parser, Subcommand};
use snow_owl_sftp::Client;
use std::path::PathBuf;
use tracing::error;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Server host
    #[arg(short = 'H', long, default_value = "localhost")]
    host: String,

    /// Server port
    #[arg(short, long, default_value = "2222")]
    port: u16,

    /// Username
    #[arg(short, long, default_value = "user")]
    username: String,

    /// Path to SSH private key
    #[arg(short = 'i', long, default_value = "~/.ssh/id_rsa")]
    identity: PathBuf,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Upload a file
    Put {
        /// Local file path
        local: PathBuf,
        /// Remote file path
        remote: String,
    },
    /// Download a file
    Get {
        /// Remote file path
        remote: String,
        /// Local file path
        local: PathBuf,
    },
    /// List directory contents
    Ls {
        /// Remote directory path
        #[arg(default_value = "/")]
        path: String,
    },
    /// Create directory
    Mkdir {
        /// Remote directory path
        path: String,
    },
    /// Remove file
    Rm {
        /// Remote file path
        path: String,
    },
    /// Remove directory
    Rmdir {
        /// Remote directory path
        path: String,
    },
    /// Rename file or directory
    Rename {
        /// Old path
        old: String,
        /// New path
        new: String,
    },
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

    // Expand tilde in identity path
    let identity_path = if args.identity.starts_with("~") {
        if let Some(home) = std::env::var_os("HOME") {
            let path_str = args.identity.to_string_lossy();
            PathBuf::from(path_str.replacen("~", &home.to_string_lossy(), 1))
        } else {
            args.identity
        }
    } else {
        args.identity
    };

    // Connect to server
    let mut client = match Client::connect(&args.host, args.port, &args.username, &identity_path).await {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to connect: {}", e);
            std::process::exit(1);
        }
    };

    // Execute command
    let result = match args.command {
        Commands::Put { local, remote } => client.put(&local, &remote).await,
        Commands::Get { remote, local } => client.get(&remote, &local).await,
        Commands::Ls { path } => {
            match client.list(&path).await {
                Ok(entries) => {
                    for (name, attrs) in entries {
                        println!("{}", name);
                        if let Some(size) = attrs.size {
                            println!("  Size: {} bytes", size);
                        }
                    }
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
        Commands::Mkdir { path } => client.mkdir(&path).await,
        Commands::Rm { path } => client.remove(&path).await,
        Commands::Rmdir { path } => client.rmdir(&path).await,
        Commands::Rename { old, new } => client.rename(&old, &new).await,
    };

    if let Err(e) = result {
        error!("Operation failed: {}", e);
        std::process::exit(1);
    }

    // Disconnect
    if let Err(e) = client.disconnect().await {
        error!("Disconnect error: {}", e);
        std::process::exit(1);
    }
}
