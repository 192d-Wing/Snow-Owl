use anyhow::Result;
use snow_owl_core::{Deployment, DeploymentStatus};
use snow_owl_db::Database;
use std::path::Path;
use uuid::Uuid;

use crate::{config, DeployCommands};

pub async fn handle(config_path: &Path, command: DeployCommands) -> Result<()> {
    let config = config::load_config(config_path).await?;
    let db = Database::new(&config.database_url).await?;

    match command {
        DeployCommands::List => list(&db).await?,
        DeployCommands::Create { machine, image } => create(&db, machine, image).await?,
        DeployCommands::Status { id } => status(&db, id).await?,
        DeployCommands::Cancel { id } => cancel(&db, id).await?,
    }

    Ok(())
}

async fn list(db: &Database) -> Result<()> {
    let deployments = db.list_deployments().await?;

    if deployments.is_empty() {
        println!("No deployments found.");
        return Ok(());
    }

    println!(
        "\n{:<36} {:<36} {:<36} {:<12}",
        "ID", "Machine", "Image", "Status"
    );
    println!("{}", "-".repeat(130));

    for deployment in deployments {
        println!(
            "{:<36} {:<36} {:<36} {:?}",
            deployment.id, deployment.machine_id, deployment.image_id, deployment.status
        );
    }

    println!();
    Ok(())
}

async fn create(db: &Database, machine_id: String, image_id: String) -> Result<()> {
    let machine_uuid = Uuid::parse_str(&machine_id)?;
    let image_uuid = Uuid::parse_str(&image_id)?;

    // Validate machine exists
    let machine = db
        .get_machine_by_id(machine_uuid)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Machine not found"))?;

    // Validate image exists
    let image = db
        .get_image_by_id(image_uuid)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Image not found"))?;

    let deployment = Deployment {
        id: Uuid::new_v4(),
        machine_id: machine_uuid,
        image_id: image_uuid,
        status: DeploymentStatus::Pending,
        started_at: chrono::Utc::now(),
        completed_at: None,
        error_message: None,
    };

    db.create_deployment(&deployment).await?;

    println!("Deployment created successfully!");
    println!("  ID: {}", deployment.id);
    println!(
        "  Machine: {} ({})",
        machine.mac_address,
        machine.hostname.unwrap_or_default()
    );
    println!("  Image: {}", image.name);
    println!("\nThe machine will receive the deployment on next boot.");

    Ok(())
}

async fn status(db: &Database, id: String) -> Result<()> {
    let deployment_id = Uuid::parse_str(&id)?;
    let deployment = db
        .get_deployment_by_id(deployment_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Deployment not found"))?;

    let machine = db.get_machine_by_id(deployment.machine_id).await?;
    let image = db.get_image_by_id(deployment.image_id).await?;

    println!("\nDeployment Status:");
    println!("  ID: {}", deployment.id);
    println!("  Status: {:?}", deployment.status);
    println!(
        "  Started: {}",
        deployment.started_at.format("%Y-%m-%d %H:%M:%S")
    );

    if let Some(completed) = deployment.completed_at {
        println!("  Completed: {}", completed.format("%Y-%m-%d %H:%M:%S"));
    }

    if let Some(machine) = machine {
        println!("\nMachine:");
        println!("  MAC: {}", machine.mac_address);
        if let Some(hostname) = machine.hostname {
            println!("  Hostname: {}", hostname);
        }
        if let Some(ip) = machine.ip_address {
            println!("  IP: {}", ip);
        }
    }

    if let Some(image) = image {
        println!("\nImage:");
        println!("  Name: {}", image.name);
        println!("  Type: {}", image.image_type);
    }

    if let Some(error) = deployment.error_message {
        println!("\nError: {}", error);
    }

    println!();
    Ok(())
}

async fn cancel(db: &Database, id: String) -> Result<()> {
    let deployment_id = Uuid::parse_str(&id)?;

    db.update_deployment_status(
        deployment_id,
        DeploymentStatus::Failed,
        Some("Cancelled by user".to_string()),
    )
    .await?;

    println!("Deployment {} cancelled.", deployment_id);
    Ok(())
}
