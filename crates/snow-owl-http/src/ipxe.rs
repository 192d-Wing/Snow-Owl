use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use snow_owl_core::{MacAddress, Machine};

use crate::AppState;

/// Generate the main iPXE boot menu
pub async fn boot_menu(State(state): State<AppState>) -> Result<impl IntoResponse, StatusCode> {
    let images = state.db.list_images().await.map_err(|e| {
        tracing::error!("Failed to list images: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let server_ip = state.config.network.server_ip;
    let http_port = state.config.http_port;

    let mut menu = String::from("#!ipxe\n\n");
    menu.push_str("# Snow-Owl Windows Deployment System\n\n");
    menu.push_str(":start\n");
    menu.push_str("menu Snow-Owl - Windows Deployment\n");
    menu.push_str("item --gap -- Available Images:\n");

    if images.is_empty() {
        menu.push_str("item --gap -- No images available\n");
    } else {
        for (idx, image) in images.iter().enumerate() {
            menu.push_str(&format!("item image{} {}\n", idx, image.name));
        }
    }

    menu.push_str("item --gap --\n");
    menu.push_str("item shell Drop to iPXE shell\n");
    menu.push_str("item reboot Reboot\n");
    menu.push_str("choose --default image0 --timeout 30000 selected || goto shell\n");
    menu.push_str("goto ${selected}\n\n");

    // Generate boot entries for each image
    for (idx, image) in images.iter().enumerate() {
        menu.push_str(&format!(":image{}\n", idx));
        menu.push_str(&format!(
            "echo Booting {} ({})\n",
            image.name, image.image_type
        ));
        menu.push_str(&generate_winpe_boot(
            server_ip,
            http_port,
            &image.id.to_string(),
        ));
        menu.push_str("\n");
    }

    menu.push_str(":shell\n");
    menu.push_str("shell\n\n");

    menu.push_str(":reboot\n");
    menu.push_str("reboot\n");

    Ok((StatusCode::OK, [("Content-Type", "text/plain")], menu))
}

/// Generate boot script for a specific MAC address
/// This can be used for machine-specific deployments
pub async fn boot_mac(
    State(state): State<AppState>,
    Path(mac): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let mac_addr: MacAddress = mac.parse().map_err(|e| {
        tracing::error!("Invalid MAC address {}: {}", mac, e);
        StatusCode::BAD_REQUEST
    })?;

    // Check if there's a pending deployment for this machine
    let machine = state.db.get_machine_by_mac(&mac_addr).await.map_err(|e| {
        tracing::error!("Failed to get machine: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if let Some(machine) = machine {
        // Update last seen
        let mut updated_machine = machine.clone();
        updated_machine.last_seen = chrono::Utc::now();
        state
            .db
            .create_or_update_machine(&updated_machine)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update machine: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Check for active deployment
        if let Some(deployment) = state
            .db
            .get_active_deployment_for_machine(machine.id)
            .await
            .unwrap()
        {
            let image = state
                .db
                .get_image_by_id(deployment.image_id)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to get image: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?
                .ok_or(StatusCode::NOT_FOUND)?;

            let server_ip = state.config.network.server_ip;
            let http_port = state.config.http_port;

            let mut script = String::from("#!ipxe\n\n");
            script.push_str(&format!("# Deployment for {}\n", mac_addr));
            script.push_str(&format!("echo Deploying image: {}\n", image.name));
            script.push_str(&generate_winpe_boot(
                server_ip,
                http_port,
                &image.id.to_string(),
            ));

            return Ok((StatusCode::OK, [("Content-Type", "text/plain")], script));
        }
    } else {
        // Register new machine
        let new_machine = Machine {
            id: uuid::Uuid::new_v4(),
            mac_address: mac_addr,
            hostname: None,
            ip_address: None,
            last_seen: chrono::Utc::now(),
            created_at: chrono::Utc::now(),
        };

        state
            .db
            .create_or_update_machine(&new_machine)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create machine: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    // No active deployment, redirect to main menu
    let redirect_script = format!(
        "#!ipxe\nchain http://{}:{}/boot.ipxe\n",
        state.config.network.server_ip, state.config.http_port
    );

    Ok((
        StatusCode::OK,
        [("Content-Type", "text/plain")],
        redirect_script,
    ))
}

fn generate_winpe_boot(server_ip: std::net::IpAddr, http_port: u16, image_id: &str) -> String {
    // For IPv6 addresses, we need to wrap them in brackets for URL formatting
    let ip_str = match server_ip {
        std::net::IpAddr::V4(ip) => ip.to_string(),
        std::net::IpAddr::V6(ip) => format!("[{}]", ip),
    };

    format!(
        r#"set base-url http://{}:{}
set image-id {}
kernel ${{base-url}}/winpe/wimboot
initrd ${{base-url}}/winpe/boot/bcd         BCD
initrd ${{base-url}}/winpe/boot/boot.sdi    boot.sdi
initrd ${{base-url}}/winpe/sources/boot.wim boot.wim
boot
"#,
        ip_str, http_port, image_id
    )
}
