use thiserror::Error;

#[derive(Error, Debug)]
pub enum SnowOwlError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("DHCP error: {0}")]
    Dhcp(String),

    #[error("TFTP error: {0}")]
    Tftp(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image not found: {0}")]
    ImageNotFound(String),

    #[error("Machine not found: {0}")]
    MachineNotFound(String),

    #[error("Deployment not found: {0}")]
    DeploymentNotFound(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, SnowOwlError>;
