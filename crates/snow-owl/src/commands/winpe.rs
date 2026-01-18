use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::info;

use crate::config;

pub async fn init(config_path: &Path, source: PathBuf, dest: Option<PathBuf>) -> Result<()> {
    let config = config::load_config(config_path).await?;
    let dest_dir = dest.unwrap_or(config.winpe_dir);

    info!("Initializing WinPE environment...");
    info!("Source: {}", source.display());
    info!("Destination: {}", dest_dir.display());

    // Create destination directories
    tokio::fs::create_dir_all(&dest_dir).await?;
    let boot_dir = dest_dir.join("boot");
    let sources_dir = dest_dir.join("sources");
    tokio::fs::create_dir_all(&boot_dir).await?;
    tokio::fs::create_dir_all(&sources_dir).await?;

    // Check if source is an ISO or directory
    if source.is_file() {
        println!("ERROR: ISO extraction not yet implemented.");
        println!("Please extract the WinPE ISO manually and provide the extracted directory.");
        println!("\nOn Linux, you can use:");
        println!("  7z x winpe.iso -o/path/to/extract");
        println!("  OR");
        println!("  mount -o loop winpe.iso /mnt/winpe");
        anyhow::bail!("ISO extraction not implemented");
    } else if source.is_dir() {
        // Copy necessary files from WinPE directory
        copy_winpe_files(&source, &dest_dir).await?;
    } else {
        anyhow::bail!("Source path does not exist");
    }

    println!("\nWinPE environment initialized successfully!");
    println!("\nNext steps:");
    println!("1. Download wimboot from: https://github.com/ipxe/wimboot/releases");
    println!("   Place it at: {}/wimboot", dest_dir.display());
    println!("2. Customize startnet.cmd in the WinPE image to run your deployment script");
    println!("3. Start the Snow-Owl server: snow-owl server");

    Ok(())
}

async fn copy_winpe_files(source: &Path, dest: &Path) -> Result<()> {
    info!("Copying WinPE files...");

    // Files to copy
    let files_to_copy = vec![
        ("boot/bcd", "boot/bcd"),
        ("boot/BCD", "boot/bcd"), // Case-insensitive fallback
        ("boot/boot.sdi", "boot/boot.sdi"),
        ("sources/boot.wim", "sources/boot.wim"),
    ];

    let mut copied = 0;

    for (src_rel, dst_rel) in files_to_copy {
        let src_path = source.join(src_rel);
        let dst_path = dest.join(dst_rel);

        if src_path.exists() {
            // Create parent directory
            if let Some(parent) = dst_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            tokio::fs::copy(&src_path, &dst_path)
                .await
                .context(format!("Failed to copy {}", src_rel))?;

            info!("Copied: {}", src_rel);
            copied += 1;
        }
    }

    if copied == 0 {
        anyhow::bail!(
            "No WinPE files found in source directory. Expected structure:\n\
             - boot/bcd (or boot/BCD)\n\
             - boot/boot.sdi\n\
             - sources/boot.wim"
        );
    }

    println!("\nCopied {} files successfully.", copied);
    Ok(())
}
