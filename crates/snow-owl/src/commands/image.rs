use anyhow::{Context, Result};
use snow_owl_core::{ImageType, WindowsImage};
use snow_owl_db::Database;
use std::path::Path;
use uuid::Uuid;

use crate::{config, ImageCommands};

pub async fn handle(config_path: &Path, command: ImageCommands) -> Result<()> {
    let config = config::load_config(config_path).await?;
    let db = Database::new(&config.database_url).await?;

    match command {
        ImageCommands::List => list(&db).await?,
        ImageCommands::Add { name, path, description } => {
            add(&db, name, path, description).await?
        }
        ImageCommands::Remove { name_or_id } => remove(&db, name_or_id).await?,
        ImageCommands::Info { name_or_id } => info(&db, name_or_id).await?,
    }

    Ok(())
}

async fn list(db: &Database) -> Result<()> {
    let images = db.list_images().await?;

    if images.is_empty() {
        println!("No images registered.");
        return Ok(());
    }

    println!("\n{:<36} {:<30} {:<8} {:<12}", "ID", "Name", "Type", "Size");
    println!("{}", "-".repeat(90));

    for image in images {
        let size_mb = image.size_bytes as f64 / 1_048_576.0;
        println!(
            "{:<36} {:<30} {:<8} {:.2} MB",
            image.id, image.name, image.image_type, size_mb
        );
    }

    println!();
    Ok(())
}

async fn add(db: &Database, name: String, path: std::path::PathBuf, description: Option<String>) -> Result<()> {
    // Determine image type from extension
    let image_type = match path.extension().and_then(|e| e.to_str()) {
        Some("wim") => ImageType::Wim,
        Some("vhd") => ImageType::Vhd,
        Some("vhdx") => ImageType::Vhdx,
        _ => anyhow::bail!("Unsupported file extension. Use .wim, .vhd, or .vhdx"),
    };

    // Check if file exists
    if !path.exists() {
        anyhow::bail!("File not found: {}", path.display());
    }

    let metadata = tokio::fs::metadata(&path).await?;

    let image = WindowsImage {
        id: Uuid::new_v4(),
        name: name.clone(),
        description,
        image_type,
        file_path: path.canonicalize()?,
        size_bytes: metadata.len(),
        created_at: chrono::Utc::now(),
        checksum: None,
    };

    db.create_image(&image).await?;

    println!("Image '{}' added successfully.", name);
    println!("ID: {}", image.id);
    println!("Type: {}", image.image_type);
    println!("Size: {:.2} MB", image.size_bytes as f64 / 1_048_576.0);

    Ok(())
}

async fn remove(db: &Database, name_or_id: String) -> Result<()> {
    let image = find_image(db, &name_or_id).await?;

    db.delete_image(image.id).await?;
    println!("Image '{}' removed successfully.", image.name);

    Ok(())
}

async fn info(db: &Database, name_or_id: String) -> Result<()> {
    let image = find_image(db, &name_or_id).await?;

    println!("\nImage Information:");
    println!("  ID: {}", image.id);
    println!("  Name: {}", image.name);
    println!("  Type: {}", image.image_type);
    println!("  Size: {:.2} MB", image.size_bytes as f64 / 1_048_576.0);
    println!("  Path: {}", image.file_path.display());
    println!("  Created: {}", image.created_at.format("%Y-%m-%d %H:%M:%S"));

    if let Some(desc) = &image.description {
        println!("  Description: {}", desc);
    }

    if let Some(checksum) = &image.checksum {
        println!("  Checksum: {}", checksum);
    }

    println!();
    Ok(())
}

async fn find_image(db: &Database, name_or_id: &str) -> Result<WindowsImage> {
    // Try as UUID first
    if let Ok(id) = Uuid::parse_str(name_or_id) {
        if let Some(image) = db.get_image_by_id(id).await? {
            return Ok(image);
        }
    }

    // Try as name
    if let Some(image) = db.get_image_by_name(name_or_id).await? {
        return Ok(image);
    }

    anyhow::bail!("Image not found: {}", name_or_id)
}
