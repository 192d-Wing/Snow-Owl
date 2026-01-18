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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Enable TLS/HTTPS
    pub enabled: bool,
    /// Path to TLS certificate file (PEM format)
    pub cert_path: PathBuf,
    /// Path to TLS private key file (PEM format)
    pub key_path: PathBuf,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub network: NetworkConfig,
    pub enable_dhcp: bool,
    pub enable_tftp: bool,
    pub tftp_root: PathBuf,
    pub http_port: u16,
    pub https_port: Option<u16>,
    pub tls: Option<TlsConfig>,
    pub images_dir: PathBuf,
    pub winpe_dir: PathBuf,
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
            tls: None, // TLS disabled by default
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
