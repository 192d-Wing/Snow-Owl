mod api;
mod ipxe;

use axum::{
    Router,
    routing::{get, post},
};
use snow_owl_core::{Result, ServerConfig, SnowOwlError};
use snow_owl_db::Database;
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tower_http::cors::CorsLayer;
use tracing::info;
use rustls::ServerConfig as RustlsServerConfig;
use rustls_pemfile::{certs, pkcs8_private_keys};

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

        // Check if TLS is configured and enabled
        if let Some(tls_config) = &self.config.tls {
            if tls_config.enabled {
                return self.run_https(app, tls_config).await;
            }
        }

        // Run HTTP server (default)
        self.run_http(app).await
    }

    async fn run_http(&self, app: Router) -> Result<()> {
        let addr = SocketAddr::new(self.config.network.server_ip, self.config.http_port);
        info!("HTTP server listening on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app)
            .await
            .map_err(|e| SnowOwlError::Http(e.to_string()))?;

        Ok(())
    }

    async fn run_https(&self, app: Router, tls_config: &snow_owl_core::TlsConfig) -> Result<()> {
        // Load TLS configuration
        let rustls_config = self.load_tls_config(tls_config)?;

        let https_port = self.config.https_port.unwrap_or(8443);
        let addr = SocketAddr::new(self.config.network.server_ip, https_port);
        info!("HTTPS server listening on https://{}", addr);
        info!("  Certificate: {}", tls_config.cert_path.display());
        info!("  Private key: {}", tls_config.key_path.display());

        // Use axum-server for TLS support
        let tls_rustls_config = axum_server::tls_rustls::RustlsConfig::from_config(Arc::new(rustls_config));

        axum_server::bind_rustls(addr, tls_rustls_config)
            .serve(app.into_make_service())
            .await
            .map_err(|e| SnowOwlError::Http(e.to_string()))?;

        Ok(())
    }

    fn load_tls_config(&self, tls_config: &snow_owl_core::TlsConfig) -> Result<RustlsServerConfig> {
        // Load certificate chain
        let cert_file = File::open(&tls_config.cert_path)
            .map_err(|e| SnowOwlError::Http(format!("Failed to open certificate file: {}", e)))?;
        let mut cert_reader = BufReader::new(cert_file);
        let cert_chain: Vec<_> = certs(&mut cert_reader)
            .collect::<std::result::Result<_, _>>()
            .map_err(|e| SnowOwlError::Http(format!("Failed to parse certificate: {}", e)))?;

        if cert_chain.is_empty() {
            return Err(SnowOwlError::Http("No certificates found in certificate file".to_string()));
        }

        // Load private key
        let key_file = File::open(&tls_config.key_path)
            .map_err(|e| SnowOwlError::Http(format!("Failed to open private key file: {}", e)))?;
        let mut key_reader = BufReader::new(key_file);
        let mut keys = pkcs8_private_keys(&mut key_reader)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| SnowOwlError::Http(format!("Failed to parse private key: {}", e)))?;

        if keys.is_empty() {
            return Err(SnowOwlError::Http("No private keys found in key file".to_string()));
        }

        let private_key = keys.remove(0);

        // Build TLS configuration
        let config = RustlsServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key.into())
            .map_err(|e| SnowOwlError::Http(format!("Failed to build TLS config: {}", e)))?;

        Ok(config)
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
