use anyhow::Result;
use snow_owl_core::MacAddress;
use snow_owl_db::Database;
use std::path::Path;
use uuid::Uuid;

use crate::{config, MachineCommands};

pub async fn handle(config_path: &Path, command: MachineCommands) -> Result<()> {
    let config = config::load_config(config_path).await?;
    let db = Database::new(&config.database_url).await?;

    match command {
        MachineCommands::List => list(&db).await?,
        MachineCommands::Info { mac_or_id } => info(&db, mac_or_id).await?,
    }

    Ok(())
}

async fn list(db: &Database) -> Result<()> {
    let machines = db.list_machines().await?;

    if machines.is_empty() {
        println!("No machines registered.");
        return Ok(());
    }

    println!(
        "\n{:<36} {:<17} {:<20} {:<15}",
        "ID", "MAC Address", "Hostname", "IP Address"
    );
    println!("{}", "-".repeat(90));

    for machine in machines {
        println!(
            "{:<36} {:<17} {:<20} {:<15}",
            machine.id,
            machine.mac_address,
            machine.hostname.as_deref().unwrap_or("-"),
            machine
                .ip_address
                .map(|ip| ip.to_string())
                .as_deref()
                .unwrap_or("-")
        );
    }

    println!();
    Ok(())
}

async fn info(db: &Database, mac_or_id: String) -> Result<()> {
    let machine = if let Ok(id) = Uuid::parse_str(&mac_or_id) {
        db.get_machine_by_id(id).await?
    } else if let Ok(mac) = mac_or_id.parse::<MacAddress>() {
        db.get_machine_by_mac(&mac).await?
    } else {
        anyhow::bail!("Invalid MAC address or UUID");
    };

    let machine = machine.ok_or_else(|| anyhow::anyhow!("Machine not found"))?;

    println!("\nMachine Information:");
    println!("  ID: {}", machine.id);
    println!("  MAC Address: {}", machine.mac_address);

    if let Some(hostname) = &machine.hostname {
        println!("  Hostname: {}", hostname);
    }

    if let Some(ip) = machine.ip_address {
        println!("  IP Address: {}", ip);
    }

    println!(
        "  Created: {}",
        machine.created_at.format("%Y-%m-%d %H:%M:%S")
    );
    println!(
        "  Last Seen: {}",
        machine.last_seen.format("%Y-%m-%d %H:%M:%S")
    );

    // Show active deployments
    if let Some(deployment) = db.get_active_deployment_for_machine(machine.id).await? {
        println!("\nActive Deployment:");
        println!("  ID: {}", deployment.id);
        println!("  Status: {:?}", deployment.status);
        println!(
            "  Started: {}",
            deployment.started_at.format("%Y-%m-%d %H:%M:%S")
        );
    }

    println!();
    Ok(())
}
