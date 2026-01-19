use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;
use uuid::Uuid;

/// MAC address representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MacAddress([u8; 6]);

impl MacAddress {
    pub fn new(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 6] {
        &self.0
    }

    pub fn to_string_colon(&self) -> String {
        format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }

    pub fn to_string_dash(&self) -> String {
        format!(
            "{:02x}-{:02x}-{:02x}-{:02x}-{:02x}-{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl std::fmt::Display for MacAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string_colon())
    }
}

impl std::str::FromStr for MacAddress {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.replace([':', '-'], "");
        if s.len() != 12 {
            anyhow::bail!("Invalid MAC address length");
        }

        let mut bytes = [0u8; 6];
        for i in 0..6 {
            bytes[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)?;
        }

        Ok(MacAddress(bytes))
    }
}

/// Deployment status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentStatus {
    Pending,
    Booting,
    Downloading,
    Installing,
    Completed,
    Failed,
}

/// Machine being deployed to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Machine {
    pub id: Uuid,
    pub mac_address: MacAddress,
    pub hostname: Option<String>,
    pub ip_address: Option<IpAddr>,
    pub last_seen: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Deployment configuration for a machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    pub id: Uuid,
    pub machine_id: Uuid,
    pub image_id: Uuid,
    pub status: DeploymentStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

/// Windows image metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsImage {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub image_type: ImageType,
    pub file_path: PathBuf,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
    pub checksum: Option<String>,
}

/// Type of Windows image
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageType {
    Wim,
    Vhd,
    Vhdx,
}

impl std::fmt::Display for ImageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageType::Wim => write!(f, "WIM"),
            ImageType::Vhd => write!(f, "VHD"),
            ImageType::Vhdx => write!(f, "VHDX"),
        }
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub interface: String,
    /// Primary server IP address (IPv4 or IPv6)
    pub server_ip: IpAddr,
    /// Optional IPv6 server address for dual-stack deployments
    pub server_ipv6: Option<Ipv6Addr>,
    /// DHCP range start (IPv4) - for external DHCP configuration reference
    pub dhcp_range_start: Ipv4Addr,
    /// DHCP range end (IPv4) - for external DHCP configuration reference
    pub dhcp_range_end: Ipv4Addr,
    /// Subnet mask (IPv4) - for external DHCP configuration reference
    pub subnet_mask: Ipv4Addr,
    /// Gateway address (IPv4 or IPv6)
    pub gateway: Option<IpAddr>,
    /// DNS servers (IPv4 or IPv6)
    pub dns_servers: Vec<IpAddr>,
}

/// TLS configuration for HTTPS
///
/// NIST Controls:
/// - SC-8: Transmission Confidentiality and Integrity
/// - SC-13: Cryptographic Protection (TLS 1.3/1.2 via Rustls)
/// - SC-23: Session Authenticity (TLS session management)
/// - IA-5(1): Password-based Authentication (certificate-based alternative)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Enable TLS/HTTPS
    pub enabled: bool,
    /// Path to TLS certificate file (PEM format)
    /// NIST SC-17: Public Key Infrastructure Certificates
    pub cert_path: PathBuf,
    /// Path to TLS private key file (PEM format)
    /// NIST SC-12: Cryptographic Key Establishment and Management
    pub key_path: PathBuf,
    /// Enable HTTP/2 via ALPN (Application-Layer Protocol Negotiation)
    /// RFC 7540: HTTP/2 protocol support
    /// NIST SC-8: Enhanced protocol efficiency while maintaining security
    #[serde(default = "default_enable_http2")]
    pub enable_http2: bool,
}

/// Default value for enable_http2 (true)
fn default_enable_http2() -> bool {
    true
}

/// Multicast TFTP configuration (RFC 2090)
///
/// RFC 2090: TFTP Multicast Option (Experimental)
/// - Allows efficient simultaneous deployment to multiple clients
/// - Reduces network bandwidth by transmitting each packet once
/// - Supports master client election and per-client ACK tracking
///
/// NIST Controls:
/// - SC-5: Denial of Service Protection (efficient bandwidth usage)
/// - CM-7: Least Functionality (optional multicast deployment)
/// - SC-7: Boundary Protection (multicast group isolation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MulticastConfig {
    /// Enable multicast TFTP deployments
    /// NIST CM-7(1): Periodic Review - can be disabled
    #[serde(default)]
    pub enabled: bool,

    /// Multicast group address (IPv4 or IPv6)
    /// RFC 2090: Clients join this group to receive data
    /// Default IPv4: 224.0.1.1 (Local Network Control Block)
    /// Default IPv6: ff12::8000:1 (Transient, Organization-Local)
    /// NIST SC-7(13): Isolation of Security Tools
    #[serde(default = "default_multicast_addr")]
    pub multicast_addr: IpAddr,

    /// Multicast port for TFTP data transmission
    /// RFC 2090: Registered port 1758 (tftp-mcast)
    /// NIST SC-7(11): Restrict Incoming Communications Traffic
    #[serde(default = "default_multicast_port")]
    pub multicast_port: u16,

    /// Maximum number of clients per multicast session
    /// NIST SC-5: Denial of Service Protection (resource limits)
    #[serde(default = "default_max_clients")]
    pub max_clients: usize,

    /// Master client election timeout in seconds
    /// RFC 2090: Time to wait for master client responses
    /// NIST SC-5(2): Capacity, Bandwidth, and Redundancy
    #[serde(default = "default_master_timeout")]
    pub master_timeout_secs: u64,

    /// Block retransmission timeout in seconds
    /// RFC 2090: Time to wait before retransmitting missed blocks
    /// NIST SC-5(2): Capacity, Bandwidth, and Redundancy
    #[serde(default = "default_retransmit_timeout")]
    pub retransmit_timeout_secs: u64,
}

/// Default multicast address (IPv4: 224.0.1.1)
fn default_multicast_addr() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(224, 0, 1, 1))
}

/// Default multicast port (1758 - RFC 2090 registered port)
fn default_multicast_port() -> u16 {
    1758
}

/// Default maximum clients per session
fn default_max_clients() -> usize {
    10
}

/// Default master client timeout (30 seconds)
fn default_master_timeout() -> u64 {
    30
}

/// Default retransmission timeout (5 seconds)
fn default_retransmit_timeout() -> u64 {
    5
}

impl Default for MulticastConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            multicast_addr: default_multicast_addr(),
            multicast_port: default_multicast_port(),
            max_clients: default_max_clients(),
            master_timeout_secs: default_master_timeout(),
            retransmit_timeout_secs: default_retransmit_timeout(),
        }
    }
}

/// Authentication configuration
///
/// NIST Controls:
/// - AC-2: Account Management (user account administration)
/// - AC-3: Access Enforcement (role-based access control)
/// - IA-2: Identification and Authentication (API key validation)
/// - IA-5: Authenticator Management (API key lifecycle)
/// - AU-2: Audit Events (authentication event logging)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Enable authentication (disabled for testing/development)
    /// NIST AC-2(1): Automated System Account Management
    pub enabled: bool,
    /// Require authentication for API endpoints
    /// NIST AC-3: Access Enforcement
    pub require_auth: bool,
}

/// User role for RBAC
///
/// NIST Controls:
/// - AC-2(7): Role-based Schemes
/// - AC-3: Access Enforcement
/// - AC-6: Least Privilege
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    /// Full administrative access
    /// NIST AC-6(2): Non-privileged Access for Nonsecurity Functions
    Admin,
    /// Can create deployments and manage machines
    /// NIST AC-6(5): Privileged Accounts
    Operator,
    /// Read-only access to view status
    /// NIST AC-6(10): Prohibit Non-privileged Users from Executing Privileged Functions
    ReadOnly,
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Admin => write!(f, "admin"),
            UserRole::Operator => write!(f, "operator"),
            UserRole::ReadOnly => write!(f, "readonly"),
        }
    }
}

/// User account
///
/// NIST Controls:
/// - AC-2: Account Management
/// - IA-2: Identification and Authentication
/// - AU-3: Content of Audit Records
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
}

/// API Key for authentication
///
/// NIST Controls:
/// - IA-5: Authenticator Management
/// - IA-5(1): Password-based Authentication (API key alternative)
/// - SC-12: Cryptographic Key Establishment and Management
/// - SC-13: Cryptographic Protection (key hashing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub key_hash: String, // SHA-256 hash of the API key
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used: Option<DateTime<Utc>>,
}

/// Server configuration
///
/// NIST Controls:
/// - CM-6: Configuration Settings (centralized configuration management)
/// - CM-7: Least Functionality (optional services can be disabled)
/// - SC-7: Boundary Protection (network segmentation via interface binding)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub network: NetworkConfig,
    /// NIST CM-7(1): Periodic Review (DHCP can be disabled)
    pub enable_dhcp: bool,
    /// NIST CM-7(1): Periodic Review (TFTP can be disabled)
    pub enable_tftp: bool,
    /// NIST AC-3: Access Enforcement (filesystem path restriction)
    pub tftp_root: PathBuf,
    /// NIST SC-7(8): Route Traffic to Authenticated Proxy Servers
    pub http_port: u16,
    /// NIST SC-8(1): Cryptographic Protection (HTTPS port)
    pub https_port: Option<u16>,
    /// NIST SC-13: Cryptographic Protection
    pub tls: Option<TlsConfig>,
    /// NIST AC-2, AC-3, IA-2: Authentication and Access Control
    pub auth: Option<AuthConfig>,
    /// RFC 2090: Multicast TFTP configuration
    /// NIST SC-5: Denial of Service Protection (efficient deployment)
    #[serde(default)]
    pub multicast: MulticastConfig,
    /// NIST AC-3: Access Enforcement (filesystem path restriction)
    pub images_dir: PathBuf,
    /// NIST AC-3: Access Enforcement (filesystem path restriction)
    pub winpe_dir: PathBuf,
    /// NIST IA-5(1): Password-based Authentication (database credentials)
    /// NIST SC-28: Protection of Information at Rest (connection string security)
    pub database_url: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                interface: "eth0".to_string(),
                server_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 100, 1)),
                server_ipv6: None,
                dhcp_range_start: Ipv4Addr::new(192, 168, 100, 100),
                dhcp_range_end: Ipv4Addr::new(192, 168, 100, 200),
                subnet_mask: Ipv4Addr::new(255, 255, 255, 0),
                gateway: Some(IpAddr::V4(Ipv4Addr::new(192, 168, 100, 1))),
                dns_servers: vec![IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))],
            },
            enable_dhcp: true,
            enable_tftp: true,
            tftp_root: PathBuf::from("/var/lib/snow-owl/tftp"),
            http_port: 8080,
            https_port: Some(8443),
            tls: None,                             // TLS disabled by default
            auth: None,                            // Auth disabled by default
            multicast: MulticastConfig::default(), // Multicast disabled by default
            images_dir: PathBuf::from("/var/lib/snow-owl/images"),
            winpe_dir: PathBuf::from("/var/lib/snow-owl/winpe"),
            database_url: "postgresql://snow_owl:password@localhost/snow_owl".to_string(),
        }
    }
}

/// iPXE boot menu entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootMenuEntry {
    pub label: String,
    pub image_id: Uuid,
    pub is_default: bool,
}
