mod commands;
mod config;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "snow-owl")]
#[command(about = "Windows deployment tool using iPXE and WinPE", long_about = None)]
#[command(version)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "/etc/snow-owl/config.toml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the deployment server (TFTP + HTTP)
    Server {
        /// Generate default configuration file
        #[arg(long)]
        init_config: bool,
    },

    /// Manage Windows images
    #[command(subcommand)]
    Image(ImageCommands),

    /// Manage deployments
    #[command(subcommand)]
    Deploy(DeployCommands),

    /// Manage machines
    #[command(subcommand)]
    Machine(MachineCommands),

    /// Initialize WinPE environment
    InitWinpe {
        /// Path to WinPE ISO or extracted directory
        source: PathBuf,

        /// Destination directory for WinPE files
        #[arg(short, long)]
        dest: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum ImageCommands {
    /// List all registered images
    List,

    /// Add a new Windows image
    Add {
        /// Image name
        name: String,

        /// Path to WIM/VHD/VHDX file
        path: PathBuf,

        /// Image description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Remove an image
    Remove {
        /// Image name or ID
        name_or_id: String,
    },

    /// Show image details
    Info {
        /// Image name or ID
        name_or_id: String,
    },
}

#[derive(Subcommand)]
enum DeployCommands {
    /// List all deployments
    List,

    /// Create a new deployment
    Create {
        /// Machine MAC address or ID
        machine: String,

        /// Image name or ID
        image: String,
    },

    /// Show deployment status
    Status {
        /// Deployment ID
        id: String,
    },

    /// Cancel a deployment
    Cancel {
        /// Deployment ID
        id: String,
    },
}

#[derive(Subcommand)]
enum MachineCommands {
    /// List all known machines
    List,

    /// Show machine details
    Info {
        /// Machine MAC address or ID
        mac_or_id: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "snow_owl=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Server { init_config } => {
            if init_config {
                commands::server::init_config(&cli.config).await?;
            } else {
                commands::server::run(&cli.config).await?;
            }
        }
        Commands::Image(cmd) => {
            commands::image::handle(&cli.config, cmd).await?;
        }
        Commands::Deploy(cmd) => {
            commands::deploy::handle(&cli.config, cmd).await?;
        }
        Commands::Machine(cmd) => {
            commands::machine::handle(&cli.config, cmd).await?;
        }
        Commands::InitWinpe { source, dest } => {
            commands::winpe::init(&cli.config, source, dest).await?;
        }
    }

    Ok(())
}
