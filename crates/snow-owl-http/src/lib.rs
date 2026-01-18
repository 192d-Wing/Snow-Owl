mod api;
mod ipxe;

use axum::{
    Router,
    routing::{get, post},
};
use snow_owl_core::{Result, ServerConfig, SnowOwlError};
use snow_owl_db::Database;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tower_http::cors::CorsLayer;
use tracing::info;

pub struct HttpServer {
    db: Arc<Database>,
    config: ServerConfig,
}

impl HttpServer {
    pub fn new(db: Arc<Database>, config: ServerConfig) -> Self {
        Self { db, config }
    }

    pub async fn run(&self) -> Result<()> {
        let app = self.create_router();

        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.http_port));
        info!("HTTP server listening on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app)
            .await
            .map_err(|e| SnowOwlError::Http(e.to_string()))?;

        Ok(())
    }

    fn create_router(&self) -> Router {
        let state = AppState {
            db: self.db.clone(),
            config: self.config.clone(),
        };

        Router::new()
            // iPXE endpoints
            .route("/boot.ipxe", get(ipxe::boot_menu))
            .route("/boot/:mac", get(ipxe::boot_mac))

            // API endpoints - Machines
            .route("/api/machines", get(api::list_machines))
            .route("/api/machines/:id", get(api::get_machine))

            // API endpoints - Images
            .route("/api/images", get(api::list_images).post(api::create_image))
            .route("/api/images/:id", get(api::get_image).delete(api::delete_image))

            // API endpoints - Deployments
            .route("/api/deployments", get(api::list_deployments).post(api::create_deployment))
            .route("/api/deployments/:id", get(api::get_deployment))
            .route("/api/deployments/:id/status", post(api::update_deployment_status))

            // Static file serving for WinPE and images
            .nest_service("/winpe", ServeDir::new(&self.config.winpe_dir))
            .nest_service("/images", ServeDir::new(&self.config.images_dir))

            // Add middleware
            .layer(CorsLayer::permissive())
            .layer(TraceLayer::new_for_http())
            .with_state(state)
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub config: ServerConfig,
}
