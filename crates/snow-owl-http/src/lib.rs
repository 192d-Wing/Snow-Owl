mod api;
pub mod auth;
mod ipxe;

use axum::{
    routing::{get, post},
    Router,
};
use rustls::ServerConfig as RustlsServerConfig;
use rustls_pemfile::{certs, pkcs8_private_keys};
use snow_owl_core::{Result, ServerConfig, SnowOwlError};
use snow_owl_db::Database;
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::info;

pub struct HttpServer {
    db: Arc<Database>,
    config: ServerConfig,
}

impl HttpServer {
    pub fn new(db: Arc<Database>, config: ServerConfig) -> Self {
        Self { db, config }
    }

    /// Start HTTP or HTTPS server based on configuration
    ///
    /// NIST Controls:
    /// - SC-8: Transmission Confidentiality and Integrity (TLS selection)
    /// - CM-7: Least Functionality (conditional TLS enablement)
    pub async fn run(&self) -> Result<()> {
        let app = self.create_router();

        // Check if TLS is configured and enabled
        // NIST SC-8(1): Cryptographic Protection - enforce encryption when configured
        if let Some(tls_config) = &self.config.tls {
            if tls_config.enabled {
                return self.run_https(app, tls_config).await;
            }
        }

        // Run HTTP server (default)
        // NIST AC-3: Access Enforcement - plaintext for iPXE compatibility
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

    /// Run HTTPS server with TLS encryption and optional HTTP/2 support
    ///
    /// RFC 7540: HTTP/2 protocol support via ALPN when enabled
    /// RFC 8446: TLS 1.3 with automatic protocol negotiation
    ///
    /// NIST Controls:
    /// - SC-8: Transmission Confidentiality and Integrity
    /// - SC-8(1): Cryptographic Protection (TLS 1.3/1.2, HTTP/2)
    /// - SC-13: Cryptographic Protection (modern cipher suites only)
    /// - SC-23: Session Authenticity (TLS session management)
    /// - AU-3: Content of Audit Records (log certificate paths)
    async fn run_https(&self, app: Router, tls_config: &snow_owl_core::TlsConfig) -> Result<()> {
        // NIST SC-12: Cryptographic Key Establishment and Management
        let rustls_config = self.load_tls_config(tls_config)?;

        let https_port = self.config.https_port.unwrap_or(8443);
        let addr = SocketAddr::new(self.config.network.server_ip, https_port);

        // NIST AU-3: Content of Audit Records - log security-relevant events
        info!("HTTPS server listening on https://{}", addr);
        info!("  Certificate: {}", tls_config.cert_path.display());
        info!("  Private key: {}", tls_config.key_path.display());

        // NIST SC-8(1): Cryptographic Protection via Rustls
        let tls_rustls_config =
            axum_server::tls_rustls::RustlsConfig::from_config(Arc::new(rustls_config));

        axum_server::bind_rustls(addr, tls_rustls_config)
            .serve(app.into_make_service())
            .await
            .map_err(|e| SnowOwlError::Http(e.to_string()))?;

        Ok(())
    }

    /// Load TLS certificates and private keys with HTTP/2 ALPN configuration
    ///
    /// RFC 7540: HTTP/2 support via ALPN (Application-Layer Protocol Negotiation)
    ///
    /// NIST Controls:
    /// - SC-12: Cryptographic Key Establishment and Management
    /// - SC-17: Public Key Infrastructure Certificates
    /// - IA-5(2): PKI-based Authentication
    /// - SI-10: Information Input Validation (certificate validation)
    /// - SC-8: Transmission Confidentiality (protocol negotiation)
    fn load_tls_config(&self, tls_config: &snow_owl_core::TlsConfig) -> Result<RustlsServerConfig> {
        // NIST SC-17: Load certificate chain from PEM file
        // NIST SI-10: Validate certificate file exists and is readable
        let cert_file = File::open(&tls_config.cert_path)
            .map_err(|e| SnowOwlError::Http(format!("Failed to open certificate file: {}", e)))?;
        let mut cert_reader = BufReader::new(cert_file);

        // NIST SI-10: Parse and validate certificate format
        let cert_chain: Vec<_> = certs(&mut cert_reader)
            .collect::<std::result::Result<_, _>>()
            .map_err(|e| SnowOwlError::Http(format!("Failed to parse certificate: {}", e)))?;

        // NIST SI-10: Verify certificate chain is not empty
        if cert_chain.is_empty() {
            return Err(SnowOwlError::Http(
                "No certificates found in certificate file".to_string(),
            ));
        }

        // NIST SC-12: Load private key from secure storage
        // NIST AC-6(9): Log All Privileged Functions (key access)
        let key_file = File::open(&tls_config.key_path)
            .map_err(|e| SnowOwlError::Http(format!("Failed to open private key file: {}", e)))?;
        let mut key_reader = BufReader::new(key_file);

        // NIST SI-10: Parse and validate private key format (PKCS#8 PEM)
        let mut keys = pkcs8_private_keys(&mut key_reader)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| SnowOwlError::Http(format!("Failed to parse private key: {}", e)))?;

        // NIST SI-10: Verify private key exists
        if keys.is_empty() {
            return Err(SnowOwlError::Http(
                "No private keys found in key file".to_string(),
            ));
        }

        let private_key = keys.remove(0);

        // NIST SC-13: Build TLS configuration with cryptographic protection
        // NIST SC-8(1): Enable modern cipher suites only (via Rustls defaults)
        // NIST IA-5(2): No client authentication required (server-only cert)
        let mut config = RustlsServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key.into())
            .map_err(|e| SnowOwlError::Http(format!("Failed to build TLS config: {}", e)))?;

        // RFC 7540: Configure HTTP/2 via ALPN (Application-Layer Protocol Negotiation)
        // NIST SC-8: Protocol negotiation for enhanced efficiency
        if tls_config.enable_http2 {
            config.alpn_protocols = vec![
                b"h2".to_vec(),       // HTTP/2 (RFC 7540)
                b"http/1.1".to_vec(), // HTTP/1.1 fallback (RFC 7230)
            ];
            info!("HTTP/2 enabled via ALPN");
        } else {
            config.alpn_protocols = vec![
                b"http/1.1".to_vec(), // HTTP/1.1 only (RFC 7230)
            ];
            info!("HTTP/2 disabled, using HTTP/1.1 only");
        }

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
            .route(
                "/api/images/:id",
                get(api::get_image).delete(api::delete_image),
            )
            // API endpoints - Deployments
            .route(
                "/api/deployments",
                get(api::list_deployments).post(api::create_deployment),
            )
            .route("/api/deployments/:id", get(api::get_deployment))
            .route(
                "/api/deployments/:id/status",
                post(api::update_deployment_status),
            )
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
