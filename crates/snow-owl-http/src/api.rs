use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use snow_owl_core::{Deployment, DeploymentStatus, ImageType, Machine, WindowsImage};
use uuid::Uuid;

use crate::AppState;

// Response types
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

// Request types
#[derive(Deserialize)]
pub struct CreateImageRequest {
    pub name: String,
    pub description: Option<String>,
    pub image_type: ImageType,
    pub file_path: String,
}

#[derive(Deserialize)]
pub struct CreateDeploymentRequest {
    pub machine_id: Uuid,
    pub image_id: Uuid,
}

#[derive(Deserialize)]
pub struct UpdateDeploymentStatusRequest {
    pub status: DeploymentStatus,
    pub error_message: Option<String>,
}

// Machine handlers
pub async fn list_machines(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<Machine>>>, StatusCode> {
    match state.db.list_machines().await {
        Ok(machines) => Ok(Json(ApiResponse::ok(machines))),
        Err(e) => {
            tracing::error!("Failed to list machines: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_machine(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Machine>>, StatusCode> {
    match state.db.get_machine_by_id(id).await {
        Ok(Some(machine)) => Ok(Json(ApiResponse::ok(machine))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get machine: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Image handlers
pub async fn list_images(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<WindowsImage>>>, StatusCode> {
    match state.db.list_images().await {
        Ok(images) => Ok(Json(ApiResponse::ok(images))),
        Err(e) => {
            tracing::error!("Failed to list images: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_image(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<WindowsImage>>, StatusCode> {
    match state.db.get_image_by_id(id).await {
        Ok(Some(image)) => Ok(Json(ApiResponse::ok(image))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get image: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn create_image(
    State(state): State<AppState>,
    Json(req): Json<CreateImageRequest>,
) -> Result<Json<ApiResponse<WindowsImage>>, StatusCode> {
    // Validate file exists
    let file_path = std::path::PathBuf::from(&req.file_path);
    if !file_path.exists() {
        return Ok(Json(ApiResponse::error(format!(
            "File not found: {}",
            req.file_path
        ))));
    }

    let metadata = match tokio::fs::metadata(&file_path).await {
        Ok(m) => m,
        Err(e) => {
            return Ok(Json(ApiResponse::error(format!(
                "Failed to read file metadata: {}",
                e
            ))));
        }
    };

    let image = WindowsImage {
        id: Uuid::new_v4(),
        name: req.name,
        description: req.description,
        image_type: req.image_type,
        file_path,
        size_bytes: metadata.len(),
        created_at: chrono::Utc::now(),
        checksum: None, // TODO: Calculate checksum
    };

    match state.db.create_image(&image).await {
        Ok(_) => Ok(Json(ApiResponse::ok(image))),
        Err(e) => {
            tracing::error!("Failed to create image: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn delete_image(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    match state.db.delete_image(id).await {
        Ok(_) => Ok(Json(ApiResponse::ok(()))),
        Err(e) => {
            tracing::error!("Failed to delete image: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Deployment handlers
pub async fn list_deployments(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<Deployment>>>, StatusCode> {
    match state.db.list_deployments().await {
        Ok(deployments) => Ok(Json(ApiResponse::ok(deployments))),
        Err(e) => {
            tracing::error!("Failed to list deployments: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_deployment(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Deployment>>, StatusCode> {
    match state.db.get_deployment_by_id(id).await {
        Ok(Some(deployment)) => Ok(Json(ApiResponse::ok(deployment))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get deployment: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn create_deployment(
    State(state): State<AppState>,
    Json(req): Json<CreateDeploymentRequest>,
) -> Result<Json<ApiResponse<Deployment>>, StatusCode> {
    // Validate machine exists
    if state
        .db
        .get_machine_by_id(req.machine_id)
        .await
        .unwrap()
        .is_none()
    {
        return Ok(Json(ApiResponse::error(format!(
            "Machine not found: {}",
            req.machine_id
        ))));
    }

    // Validate image exists
    if state
        .db
        .get_image_by_id(req.image_id)
        .await
        .unwrap()
        .is_none()
    {
        return Ok(Json(ApiResponse::error(format!(
            "Image not found: {}",
            req.image_id
        ))));
    }

    let deployment = Deployment {
        id: Uuid::new_v4(),
        machine_id: req.machine_id,
        image_id: req.image_id,
        status: DeploymentStatus::Pending,
        started_at: chrono::Utc::now(),
        completed_at: None,
        error_message: None,
    };

    match state.db.create_deployment(&deployment).await {
        Ok(_) => Ok(Json(ApiResponse::ok(deployment))),
        Err(e) => {
            tracing::error!("Failed to create deployment: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn update_deployment_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateDeploymentStatusRequest>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    match state
        .db
        .update_deployment_status(id, req.status, req.error_message)
        .await
    {
        Ok(_) => Ok(Json(ApiResponse::ok(()))),
        Err(e) => {
            tracing::error!("Failed to update deployment status: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
