use anyhow::{Context, Result};
use snow_owl_core::ServerConfig;
use std::path::Path;
use tokio::fs;

pub async fn load_config(path: &Path) -> Result<ServerConfig> {
    let contents = fs::read_to_string(path)
        .await
        .context("Failed to read configuration file")?;

    let config: ServerConfig =
        toml::from_str(&contents).context("Failed to parse configuration file")?;

    Ok(config)
}

pub async fn save_config(path: &Path, config: &ServerConfig) -> Result<()> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let contents = toml::to_string_pretty(config).context("Failed to serialize configuration")?;

    fs::write(path, contents)
        .await
        .context("Failed to write configuration file")?;

    Ok(())
}
