# Snow-Owl 🦉

A high-performance Windows deployment tool using iPXE and WinPE, written in Rust.

Snow-Owl provides a complete PXE boot infrastructure for deploying Windows images to bare-metal machines. It features TFTP and HTTP servers for network booting, a REST API for deployment management, and automated WinPE-based deployment workflows.

## Features

- 🚀 **High Performance**: Built with Rust for speed and safety
- 🔧 **Complete PXE Stack**: TFTP and HTTP servers for network booting
- 🪟 **Windows Deployment**: Support for WIM, VHD, and VHDX image formats
- 🌐 **REST API**: Full API for automation and integration
- 📊 **Deployment Tracking**: PostgreSQL database for tracking machines and deployments
- 🔄 **Dynamic Boot Menus**: iPXE-based boot menus generated on-the-fly
- 🔐 **Authentication & Authorization**: API key-based auth with role-based access control (RBAC)
- 🔒 **TLS/HTTPS Support**: Optional encrypted communications (RFC 8446 compliant)
- 🚄 **HTTP/2 Support**: Optional HTTP/2 via ALPN for improved API performance (RFC 7540 compliant)
- 🌐 **IPv6 Support**: Full dual-stack IPv4/IPv6 networking (RFC 2460 compliant)
- 📡 **Multicast TFTP**: Efficient simultaneous deployment to multiple clients (RFC 2090 compliant)
- 📊 **SIEM Integration**: Comprehensive audit logging with JSON format for security monitoring
- 🛡️ **Security**: Safe Rust code with NIST SP 800-53 security controls

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Snow-Owl Server                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ TFTP Server  │  │ HTTP Server  │  │   Database   │     │
│  │   (Port 69)  │  │  (Port 8080) │  │ (PostgreSQL) │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│         │                 │                   │             │
│         │                 │                   │             │
│  ┌──────▼─────────────────▼───────────────────▼──────┐     │
│  │              Core Application Logic               │     │
│  │  - Image Management                                │     │
│  │  - Deployment Orchestration                        │     │
│  │  - Machine Tracking                                │     │
│  │  - iPXE Menu Generation                            │     │
│  └───────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
              ┌────────────────────────┐
              │   Target Machines      │
              │  (PXE Boot → WinPE)    │
              └────────────────────────┘
```

## Requirements

### Server Requirements

- Linux server (Ubuntu 20.04+ recommended)
- Rust 1.70+ (for building from source)
- Root privileges (for TFTP port 69)
- Network interface on deployment network

### Network Requirements

- DHCP server (can be external) configured for PXE boot
- DHCP option 66: TFTP server IP (Snow-Owl server)
- DHCP option 67: Boot filename (`undionly.kpxe` or `ipxe.efi`)

### Windows Requirements

- Windows ADK (Assessment and Deployment Kit) for creating WinPE
- Windows installation media or custom WIM images

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/Wing/Snow-Owl.git
cd Snow-Owl

# Build the project
cargo build --release

# Install (optional)
sudo cp target/release/snow-owl /usr/local/bin/
```

### Configuration

1. Generate default configuration:

```bash
sudo snow-owl server --init-config
```

1. Edit the configuration file at `/etc/snow-owl/config.toml`:

```toml
[network]
interface = "eth0"
server_ip = "192.168.100.1"
dhcp_range_start = "192.168.100.100"
dhcp_range_end = "192.168.100.200"
subnet_mask = "255.255.255.0"
gateway = "192.168.100.1"
dns_servers = ["8.8.8.8"]

enable_dhcp = false  # Use external DHCP
enable_tftp = true
tftp_root = "/var/lib/snow-owl/tftp"
http_port = 8080
images_dir = "/var/lib/snow-owl/images"
winpe_dir = "/var/lib/snow-owl/winpe"
database_url = "postgresql://snow_owl:password@localhost/snow_owl"
```

#### IPv6 Configuration

Snow-Owl supports both IPv4 and IPv6 deployments. For IPv6 or dual-stack configurations:

```toml
[network]
interface = "eth0"
# IPv6-only configuration
server_ip = "fd00::1"
server_ipv6 = "fd00::1"  # Optional: for dual-stack
dhcp_range_start = "192.168.100.100"  # For reference only
dhcp_range_end = "192.168.100.200"
subnet_mask = "255.255.255.0"
gateway = "fd00::1"  # IPv6 gateway
dns_servers = ["2001:4860:4860::8888", "2001:4860:4860::8844"]  # Google Public DNS IPv6

enable_dhcp = false
enable_tftp = true
tftp_root = "/var/lib/snow-owl/tftp"
http_port = 8080
images_dir = "/var/lib/snow-owl/images"
winpe_dir = "/var/lib/snow-owl/winpe"
database_url = "postgresql://snow_owl:password@localhost/snow_owl"
```

**Note:** For dual-stack deployments, set `server_ip` to your primary IP (IPv4 or IPv6) and optionally configure `server_ipv6` for additional IPv6 support. The TFTP and HTTP servers will bind to the configured `server_ip` address.

### PostgreSQL Setup

Snow-Owl requires PostgreSQL for storing deployment data.

**Install PostgreSQL:**

```bash
# Ubuntu/Debian
sudo apt install postgresql postgresql-contrib

# CentOS/RHEL
sudo yum install postgresql-server postgresql-contrib
sudo postgresql-setup initdb
sudo systemctl start postgresql
```

**Create Database and User:**

```bash
sudo -u postgres psql
```

```sql
CREATE DATABASE snow_owl;
CREATE USER snow_owl WITH PASSWORD 'your_secure_password';
GRANT ALL PRIVILEGES ON DATABASE snow_owl TO snow_owl;
\q
```

**Update Configuration:**

Edit `/etc/snow-owl/config.toml` and set the `database_url`:

```toml
database_url = "postgresql://snow_owl:your_secure_password@localhost/snow_owl"
```

### TLS/HTTPS Configuration

Snow-Owl supports TLS/HTTPS for secure API access and encrypted boot script delivery. TLS is optional and disabled by default.

#### Generating TLS Certificates

**Option 1: Self-Signed Certificate (for testing)**

```bash
# Generate private key and self-signed certificate
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout /etc/snow-owl/server-key.pem \
  -out /etc/snow-owl/server-cert.pem \
  -days 365 \
  -subj "/CN=snow-owl.example.com"

# Set appropriate permissions
chmod 600 /etc/snow-owl/server-key.pem
chmod 644 /etc/snow-owl/server-cert.pem
```

**Option 2: Let's Encrypt (for production)**

```bash
# Install certbot
sudo apt install certbot  # Ubuntu/Debian

# Obtain certificate (HTTP-01 challenge)
sudo certbot certonly --standalone -d snow-owl.example.com

# Certificates will be in /etc/letsencrypt/live/snow-owl.example.com/
# Link them to Snow-Owl's expected location
sudo ln -s /etc/letsencrypt/live/snow-owl.example.com/fullchain.pem /etc/snow-owl/server-cert.pem
sudo ln -s /etc/letsencrypt/live/snow-owl.example.com/privkey.pem /etc/snow-owl/server-key.pem
```

**Option 3: Use Your Organization's CA**

Place your certificate and private key in PEM format at:

- Certificate: `/etc/snow-owl/server-cert.pem`
- Private key: `/etc/snow-owl/server-key.pem`

#### Enable TLS in Configuration

Edit `/etc/snow-owl/config.toml`:

```toml
[network]
interface = "eth0"
server_ip = "192.168.100.1"
# ... other network settings ...

enable_dhcp = false
enable_tftp = true
tftp_root = "/var/lib/snow-owl/tftp"
http_port = 8080        # Still available for iPXE (unencrypted)
https_port = 8443       # HTTPS port for API and secure access
images_dir = "/var/lib/snow-owl/images"
winpe_dir = "/var/lib/snow-owl/winpe"
database_url = "postgresql://snow_owl:password@localhost/snow_owl"

[tls]
enabled = true
cert_path = "/etc/snow-owl/server-cert.pem"
key_path = "/etc/snow-owl/server-key.pem"
enable_http2 = true  # Enable HTTP/2 via ALPN (default: true)
```

**Notes:**

- When TLS is enabled, the HTTP server runs on the HTTPS port only
- iPXE boot scripts are served over HTTPS
- API endpoints are encrypted
- TFTP remains unencrypted (required for network boot)
- For production, use certificates from a trusted CA or Let's Encrypt
- **HTTP/2 Support**: When `enable_http2 = true`, the server advertises HTTP/2 support via ALPN
  - Clients can negotiate HTTP/2 or HTTP/1.1 during TLS handshake
  - Provides better performance for API clients with multiplexing and header compression
  - Automatically falls back to HTTP/1.1 for clients that don't support HTTP/2
  - HTTP/2 only available with HTTPS (plain HTTP uses HTTP/1.1)

### Multicast TFTP Deployment

Snow-Owl supports RFC 2090 multicast TFTP for efficient simultaneous deployment to multiple clients. This feature allows a single file transfer to be received by multiple machines simultaneously, significantly reducing network bandwidth usage.

#### Benefits of Multicast Deployment

- **Bandwidth Efficiency**: Each data packet is transmitted only once, regardless of client count
- **Scalability**: Deploy to 10+ machines simultaneously without network congestion
- **Coordinated Transfers**: Master client election ensures synchronized deployment
- **Selective Retransmission**: Only missed blocks are retransmitted to specific clients

#### Enable Multicast TFTP

Edit `/etc/snow-owl/config.toml`:

```toml
[multicast]
enabled = true
multicast_addr = "224.0.1.1"  # IPv4 multicast group (default)
# multicast_addr = "ff12::8000:1"  # IPv6 multicast group (alternative)
multicast_port = 1758  # RFC 2090 registered port (default)
max_clients = 10  # Maximum clients per session (default)
master_timeout_secs = 30  # Master client election timeout (default)
retransmit_timeout_secs = 5  # Block retransmission timeout (default)
```

#### Configuration Options

| Option | Description | Default | Valid Range |
|--------|-------------|---------|-------------|
| `enabled` | Enable multicast TFTP | `false` | `true`/`false` |
| `multicast_addr` | Multicast group address | `224.0.1.1` | IPv4/IPv6 multicast |
| `multicast_port` | Multicast port | `1758` | `1024-65535` |
| `max_clients` | Max clients per session | `10` | `1-100` |
| `master_timeout_secs` | Master election timeout | `30` | `10-300` |
| `retransmit_timeout_secs` | Retransmission timeout | `5` | `1-60` |

#### How Multicast Works (RFC 2090)

1. **Session Creation**: First client requests a file with multicast option
2. **Master Election**: First client becomes the "master client"
3. **Group Join**: Additional clients join the same multicast session
4. **Data Transmission**: Server sends data packets to multicast group
5. **ACK Coordination**: Each client acknowledges received blocks
6. **Selective Retransmission**: Server retransmits missed blocks as needed
7. **Completion**: Transfer completes when all clients have received all blocks

#### Network Requirements

**IPv4 Multicast:**

- Multicast address range: `224.0.0.0` - `239.255.255.255`
- Default: `224.0.1.1` (Local Network Control Block)
- Requires IGMP support on network switches

**IPv6 Multicast:**

- Multicast address range: `ff00::/8`
- Default: `ff12::8000:1` (Transient, Organization-Local)
- Requires MLD support on network switches

**IMPORTANT**: Ensure your network infrastructure supports multicast:

- Switches must support IGMP snooping (IPv4) or MLD snooping (IPv6)
- Routers must support multicast routing if deploying across subnets
- Firewalls must allow multicast traffic on port 1758

#### Usage Example

Standard TFTP clients can request multicast transfers by including the multicast option:

```bash
# Example: iPXE script with multicast
#!ipxe
dhcp
set tftp-opts multicast
chain tftp://${next-server}/winpe/boot.wim
```

#### Security Considerations (NIST Controls)

- **SC-5**: Denial of Service Protection - Efficient bandwidth usage prevents network saturation
- **SC-7**: Boundary Protection - Multicast groups provide network isolation
- **AC-3**: Access Enforcement - Session membership control and max client limits
- **AU-2**: Audit Events - Comprehensive logging of multicast sessions and client activity

### Authentication and Authorization

Snow-Owl includes comprehensive API key-based authentication with role-based access control (RBAC) to secure your deployment infrastructure. Authentication is optional and can be enabled in the configuration.

#### Authentication Overview

The authentication system provides:

- **API Key Authentication**: Secure Bearer token authentication for API access
- **Role-Based Access Control (RBAC)**: Three privilege levels (Admin, Operator, ReadOnly)
- **Secure Key Storage**: SHA-256 hashed API keys stored in the database
- **Audit Logging**: Security events logged for compliance (NIST AU-2, AU-3)
- **Key Expiration**: Optional expiration dates for API keys

**NIST SP 800-53 Controls Implemented:**

- AC-2: Account Management
- AC-3: Access Enforcement
- AC-6: Least Privilege
- IA-2: Identification and Authentication
- IA-5: Authenticator Management
- AU-2: Audit Events
- SC-12: Cryptographic Key Establishment
- SC-13: Cryptographic Protection

#### Enable Authentication

Edit `/etc/snow-owl/config.toml`:

```toml
[auth]
enabled = true
require_auth = true  # Set to false to make authentication optional
```

#### User Roles

Snow-Owl implements three privilege levels:

| Role | Permissions | Use Case |
|------|-------------|----------|
| **Admin** | Full access to all operations | System administrators |
| **Operator** | Create/manage deployments and images | Deployment engineers |
| **ReadOnly** | View-only access to all resources | Auditors, monitoring systems |

#### Creating Users

**Create the first admin user:**

```bash
snow-owl user create admin --role admin
```

**Create additional users:**

```bash
# Create an operator
snow-owl user create deploy-eng --role operator

# Create a read-only user
snow-owl user create auditor --role readonly
```

**List all users:**

```bash
snow-owl user list
```

**View user details:**

```bash
snow-owl user info admin
```

#### Managing API Keys

**Generate an API key:**

```bash
snow-owl api-key create admin --name "Production API Key"

# With expiration (90 days)
snow-owl api-key create deploy-eng --name "Temporary Key" --expires 90
```

Output:

```
✓ API key created successfully

  User: admin
  Name: Production API Key
  Key ID: a1b2c3d4-e5f6-7890-abcd-ef1234567890

  API Key: so_a1b2c3d4-e5f6-7890-abcd-ef1234567890

⚠ IMPORTANT: Store this API key securely!
  This is the only time you will see the full key.
  The key is stored as a hash and cannot be recovered.
```

**List user's API keys:**

```bash
snow-owl api-key list admin
```

**Revoke an API key:**

```bash
snow-owl api-key revoke a1b2c3d4-e5f6-7890-abcd-ef1234567890
```

#### Using API Keys

All API requests must include the API key in the `Authorization` header:

**List machines:**

```bash
curl -H "Authorization: Bearer so_a1b2c3d4-e5f6-7890-abcd-ef1234567890" \
  http://192.168.100.1:8080/api/machines
```

**Create a deployment:**

```bash
curl -X POST http://192.168.100.1:8080/api/deployments \
  -H "Authorization: Bearer so_a1b2c3d4-e5f6-7890-abcd-ef1234567890" \
  -H "Content-Type: application/json" \
  -d '{
    "machine_id": "uuid-of-machine",
    "image_id": "uuid-of-image"
  }'
```

**With HTTPS:**

```bash
curl -H "Authorization: Bearer so_a1b2c3d4-e5f6-7890-abcd-ef1234567890" \
  https://192.168.100.1:8443/api/images
```

#### Permission Requirements

| Endpoint | Admin | Operator | ReadOnly |
|----------|-------|----------|----------|
| GET /api/machines | ✓ | ✓ | ✓ |
| GET /api/images | ✓ | ✓ | ✓ |
| POST /api/images | ✓ | ✓ | ✗ |
| DELETE /api/images/:id | ✓ | ✓ | ✗ |
| POST /api/deployments | ✓ | ✓ | ✗ |
| GET /api/deployments | ✓ | ✓ | ✓ |

#### Security Best Practices

1. **Secure Key Storage**: Store API keys in environment variables or secure vaults
2. **Key Rotation**: Regularly rotate API keys, especially for production systems
3. **Least Privilege**: Use ReadOnly keys for monitoring and reporting
4. **Set Expiration**: Use `--expires` for temporary or contractor access
5. **Audit Regularly**: Review `snow-owl user list` and API key usage
6. **Revoke Immediately**: Revoke compromised keys with `api-key revoke`
7. **Use HTTPS**: Always use HTTPS in production to protect API keys in transit

## Setup Guide

### 1. Prepare iPXE Boot Files

Download iPXE boot files and place them in the TFTP root:

```bash
# Create TFTP directory
sudo mkdir -p /var/lib/snow-owl/tftp

# Download iPXE (BIOS)
sudo wget -O /var/lib/snow-owl/tftp/undionly.kpxe \
    https://boot.ipxe.org/undionly.kpxe

# Download iPXE (UEFI)
sudo wget -O /var/lib/snow-owl/tftp/ipxe.efi \
    https://boot.ipxe.org/ipxe.efi

# Create iPXE chainload script
sudo tee /var/lib/snow-owl/tftp/boot.ipxe <<EOF
#!ipxe
chain http://192.168.100.1:8080/boot.ipxe
EOF
```

### 2. Prepare WinPE

#### Create WinPE on Windows

1. Install Windows ADK with WinPE add-on
2. Open "Deployment and Imaging Tools Environment" as Administrator
3. Create WinPE:

```cmd
copype amd64 C:\WinPE
```

1. Customize WinPE with deployment scripts:

```cmd
Dism /Mount-Image /ImageFile:"C:\WinPE\media\sources\boot.wim" /Index:1 /MountDir:"C:\WinPE\mount"

REM Add PowerShell support
Dism /Add-Package /Image:"C:\WinPE\mount" /PackagePath:"C:\Program Files (x86)\Windows Kits\10\Assessment and Deployment Kit\Windows Preinstallation Environment\amd64\WinPE_OCs\WinPE-PowerShell.cab"

REM Copy deployment scripts (from Snow-Owl repository)
xcopy /E /I Snow-Owl\scripts\winpe\* C:\WinPE\mount\Deploy\

REM Update startnet.cmd
copy /Y Snow-Owl\scripts\winpe\startnet.cmd C:\WinPE\mount\Windows\System32\

REM Unmount and commit
Dism /Unmount-Image /MountDir:"C:\WinPE\mount" /Commit
```

#### Transfer WinPE to Linux Server

```bash
# Copy WinPE files to Snow-Owl server
scp -r C:\WinPE\media\* user@server:/tmp/winpe/

# On the server, initialize WinPE environment
sudo snow-owl init-winpe /tmp/winpe

# Download wimboot
sudo wget -O /var/lib/snow-owl/winpe/wimboot \
    https://github.com/ipxe/wimboot/releases/latest/download/wimboot
```

Alternatively, use the provided script:

```bash
# On Linux (requires wimlib-tools)
sudo apt install wimlib-tools
./scripts/customize-winpe.sh /path/to/boot.wim http://192.168.100.1:8080
```

### 3. Configure DHCP Server

Configure your existing DHCP server for PXE boot:

**ISC DHCP Server** (`/etc/dhcp/dhcpd.conf`):

```
subnet 192.168.100.0 netmask 255.255.255.0 {
    range 192.168.100.100 192.168.100.200;
    option routers 192.168.100.1;
    option domain-name-servers 8.8.8.8;

    # PXE boot configuration
    next-server 192.168.100.1;  # Snow-Owl TFTP server

    # Boot file based on client architecture
    if exists user-class and option user-class = "iPXE" {
        filename "http://192.168.100.1:8080/boot.ipxe";
    } elsif option arch = 00:07 or option arch = 00:09 {
        filename "ipxe.efi";  # UEFI
    } else {
        filename "undionly.kpxe";  # BIOS
    }
}
```

**dnsmasq**:

```
dhcp-range=192.168.100.100,192.168.100.200,12h
dhcp-boot=tag:bios,undionly.kpxe
dhcp-boot=tag:efi,ipxe.efi
dhcp-option-force=209,boot.ipxe
dhcp-option-force=210,http://192.168.100.1:8080/
```

### 4. Start Snow-Owl Server

```bash
sudo snow-owl server
```

## Usage

### Managing Images

#### Add a Windows Image

```bash
# Add a WIM image
snow-owl image add \
    "Windows Server 2022" \
    /path/to/install.wim \
    --description "Windows Server 2022 Datacenter"

# Add a VHD image
snow-owl image add \
    "Windows 10 Enterprise" \
    /path/to/win10.vhdx \
    --description "Windows 10 Enterprise with apps"
```

#### List Images

```bash
snow-owl image list
```

#### View Image Details

```bash
snow-owl image info "Windows Server 2022"
```

#### Remove an Image

```bash
snow-owl image remove "Windows Server 2022"
```

### Managing Deployments

#### List Machines

```bash
# List all discovered machines
snow-owl machine list
```

#### Create a Deployment

```bash
# Deploy to a specific machine
snow-owl deploy create \
    00:11:22:33:44:55 \
    "Windows Server 2022"
```

#### Check Deployment Status

```bash
snow-owl deploy status <deployment-id>
```

#### List All Deployments

```bash
snow-owl deploy list
```

### Using the REST API

The HTTP server exposes a REST API on port 8080 (configurable). When authentication is enabled, all API requests require an API key in the `Authorization` header.

#### List Images

```bash
# Without authentication
curl http://192.168.100.1:8080/api/images

# With authentication
curl -H "Authorization: Bearer so_your-api-key-here" \
    http://192.168.100.1:8080/api/images
```

#### Create a Deployment

```bash
curl -X POST http://192.168.100.1:8080/api/deployments \
    -H "Authorization: Bearer so_your-api-key-here" \
    -H "Content-Type: application/json" \
    -d '{
        "machine_id": "uuid-of-machine",
        "image_id": "uuid-of-image"
    }'
```

#### Get Boot Menu

```bash
# Main boot menu (authentication not required for boot scripts)
curl http://192.168.100.1:8080/boot.ipxe

# Machine-specific boot (by MAC address)
curl http://192.168.100.1:8080/boot/00:11:22:33:44:55
```

## Deployment Workflow

1. **Machine boots via PXE**
   - DHCP assigns IP and provides iPXE boot file
   - Machine downloads iPXE from TFTP server

2. **iPXE chainloads to Snow-Owl**
   - iPXE requests boot menu from HTTP server
   - Server generates dynamic menu with available images

3. **User selects image (or automatic deployment)**
   - iPXE loads WinPE via HTTP
   - WinPE boots on the target machine

4. **WinPE runs deployment script**
   - Downloads Windows image from Snow-Owl
   - Partitions disk
   - Applies image to disk
   - Installs bootloader
   - Reports status back to Snow-Owl

5. **Machine reboots into installed Windows**

## Directory Structure

```
/var/lib/snow-owl/
├── tftp/                    # TFTP root directory
│   ├── undionly.kpxe       # iPXE for BIOS
│   ├── ipxe.efi            # iPXE for UEFI
│   └── boot.ipxe           # iPXE chainload script
├── winpe/                   # WinPE files
│   ├── wimboot             # wimboot bootloader
│   ├── boot/
│   │   ├── bcd            # Boot configuration
│   │   └── boot.sdi        # Boot SDI
│   └── sources/
│       └── boot.wim        # WinPE image
└── images/                  # Windows images
    ├── server2022.wim
    └── win10.vhdx
```

## Troubleshooting

### Machine doesn't PXE boot

- Check DHCP server configuration (options 66 and 67)
- Verify BIOS/UEFI boot order
- Ensure network cable is connected
- Check TFTP server is running: `sudo netstat -ulnp | grep :69`

### iPXE fails to chainload

- Verify HTTP server is accessible: `curl http://192.168.100.1:8080/boot.ipxe`
- Check firewall rules: `sudo ufw allow 8080/tcp`
- Review Snow-Owl logs

### WinPE doesn't boot

- Verify wimboot and WinPE files exist
- Check HTTP server can serve large files
- Ensure WinPE is properly customized with deployment scripts

### Deployment fails

- Check WinPE logs at `X:\deploy.log`
- Verify image file exists and is accessible
- Ensure target machine has sufficient disk space
- Check network connectivity from WinPE

### View Snow-Owl Logs

```bash
# Run with debug logging
RUST_LOG=debug sudo snow-owl server
```

## Development

### Project Structure

```
Snow-Owl/
├── crates/
│   ├── snow-owl/          # Main CLI application
│   ├── snow-owl-core/     # Core types and utilities
│   ├── snow-owl-db/       # Database layer (PostgreSQL)
│   ├── snow-owl-tftp/     # TFTP server implementation
│   └── snow-owl-http/     # HTTP server and REST API
├── scripts/
│   ├── winpe/             # WinPE deployment scripts
│   └── customize-winpe.sh # WinPE customization helper
└── Cargo.toml             # Workspace configuration
```

## SIEM Integration and Audit Logging

Snow-Owl TFTP provides comprehensive structured audit logging designed for Security Information and Event Management (SIEM) integration. All security-relevant events are logged in JSON format for easy parsing and analysis.

### Features

- **Structured JSON Logging**: Machine-parsable audit events for SIEM ingestion
- **Comprehensive Event Coverage**: Server lifecycle, file access, security violations, and multicast sessions
- **Performance Metrics**: Throughput, transfer duration, and block-level timing statistics
- **Correlation IDs**: Transaction tracing across related events (read_request → transfer_started → transfer_completed)
- **NIST Compliance**: Satisfies AU-2, AU-3, AU-6, AU-9, AU-12 requirements
- **Multiple SIEM Platforms**: Integration guides for Splunk, ELK, Datadog, CloudWatch, Fluentd

### Quick Start

Audit logging is **enabled by default** with JSON format:

```toml
[logging]
format = "json"
file = "/var/log/snow-owl/tftp-audit.json"
audit_enabled = true
level = "info"
```

Create the log directory and start the server:

```bash
sudo mkdir -p /var/log/snow-owl
sudo chown snow-owl:snow-owl /var/log/snow-owl
sudo chmod 750 /var/log/snow-owl
snow-owl-tftp --config /etc/snow-owl/tftp.toml
```

### Event Types

**Server Lifecycle**: `server_started`, `server_stopped`, `config_reload`

**File Access**: `read_request`, `transfer_started`, `transfer_completed`, `transfer_failed`

**Security Violations**: `path_traversal_attempt`, `file_size_limit_exceeded`, `read_denied`, `write_request_denied`, `symlink_access_denied`

**Multicast Sessions**: `multicast_session_created`, `multicast_client_joined`, `multicast_session_completed`

### Example Audit Event

```json
{
  "event_type": "transfer_completed",
  "timestamp": "2026-01-18T10:05:25.789Z",
  "hostname": "tftp-01",
  "service": "snow-owl-tftp",
  "severity": "info",
  "client_addr": "192.168.1.100:54321",
  "filename": "firmware.bin",
  "bytes_transferred": 1048576,
  "blocks_sent": 1024,
  "duration_ms": 2333,
  "throughput_bps": 449235,
  "avg_block_time_ms": 2.278,
  "correlation_id": "18f2a1b3c4d-192-168-1-100-54321-a3f2d8e1"
}
```

### SIEM Platform Integration

**Splunk**: Use Filebeat or HTTP Event Collector (HEC) for log ingestion

**ELK Stack**: Configure Logstash with JSON codec and GeoIP enrichment

**Datadog**: Use the Datadog agent with JSON log parsing

**AWS CloudWatch**: Configure CloudWatch agent for log collection

**Fluentd**: Use tail input with JSON parser

### Security Alerting

Monitor for these critical events:

- **Path Traversal Attempts** (`path_traversal_attempt`) - Immediate alert and IP block
- **Repeated Access Denials** (`read_denied` >5/min from same IP) - Security team alert
- **Write Request Attempts** (`write_request_denied`) - Alert on read-only server
- **File Size Limit Violations** (`file_size_limit_exceeded`) - Review limits
- **Symlink Access Attempts** (`symlink_access_denied`) - Potential security probe

### Performance Monitoring

Track transfer performance metrics:

```
# Average throughput by file
event_type=transfer_completed | stats avg(throughput_bps) by filename

# Slow transfer detection (< 100 KB/s)
event_type=transfer_completed throughput_bps:<102400

# Transfer completion rates
event_type=transfer_completed | stats count by filename
```

### Compliance

The audit logs satisfy:

- **NIST 800-53**: AU-2, AU-3, AU-6, AU-9, AU-12
- **STIG**: V-222563, V-222564, V-222565
- **PCI-DSS**: 10.2, 10.3, 10.5
- **HIPAA**: 164.312(b) - Audit controls

### Detailed Documentation

For comprehensive SIEM integration guides, example queries, dashboard configurations, and troubleshooting, see:

- [SIEM Integration Guide](crates/snow-owl-tftp/SIEM-INTEGRATION.md) - Complete integration documentation
- [Security Compliance](crates/snow-owl-tftp/SECURITY-COMPLIANCE.md) - NIST 800-53 control mapping

## Standalone TFTP Server (snow-owl-tftp)

The TFTP server now runs as a standalone binary with a TOML config file.

### Config File (TOML Schema)

Default path: `/etc/snow-owl/tftp.toml`

```toml
root_dir = "/var/lib/snow-owl/tftp"
bind_addr = "[::]:69"

[logging]
level = "info"
format = "text" # "text" or "json"
# file = "/var/log/snow-owl/tftp.log"

[multicast]
enabled = false
multicast_addr = "ff12::8000:1"
multicast_ip_version = "v6"
multicast_port = 1758
max_clients = 10
master_timeout_secs = 30
retransmit_timeout_secs = 5
```

### Validation Rules

- `root_dir` must be an absolute path and must exist as a directory
- `root_dir` must be readable by the server process
- `bind_addr` must include a non-zero port
- `multicast.multicast_port` must be in `1024..=65535`
- `multicast.multicast_addr` must match `multicast.multicast_ip_version`
- `logging.file` parent directory must exist and be writable

### Init and Run

```bash
# Write a default config file
sudo snow-owl-tftp --init-config

# Run the server with the config file
sudo snow-owl-tftp --config /etc/snow-owl/tftp.toml

# Validate config without binding to the port
snow-owl-tftp --config /etc/snow-owl/tftp.toml --check-config
```

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run -- server
```

### Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

### RFC Compliance

Snow-Owl adheres to relevant IETF and networking standards. For detailed information about RFC compliance, including TFTP protocol implementation, network standards, and security considerations, see [RFC_COMPLIANCE.md](RFC_COMPLIANCE.md).

## License

Snow-Owl is dual-licensed under MIT OR Apache-2.0.

## Acknowledgments

- [iPXE](https://ipxe.org/) - Network boot firmware
- [wimboot](https://github.com/ipxe/wimboot) - Windows Imaging boot loader
- Built with [Rust](https://www.rust-lang.org/) 🦀

## Security

### Reporting Security Issues

Please report security vulnerabilities to the repository maintainers privately.

### Security Considerations

- Snow-Owl requires root privileges for TFTP (port 69)
- TFTP has built-in path traversal protection (NIST AC-3, SI-10)
- Database uses PostgreSQL with parameterized queries (SQL injection prevention)
- **Authentication**: API key-based authentication with SHA-256 hashing
- **Authorization**: Role-based access control (Admin, Operator, ReadOnly)
- **Encryption**: Optional TLS 1.3/1.2 support for HTTPS API access
- **IPv6**: Full dual-stack support for modern networks
- **Audit Logging**: Security events logged for compliance (NIST AU-2, AU-3)
- **NIST Compliance**: Implements 21+ NIST SP 800-53 security controls
- Consider using VLANs to isolate deployment network
- For production: Enable authentication, use HTTPS, and deploy behind a firewall

## Roadmap

- ✅ Add authentication and authorization
- ✅ TLS/HTTPS support for encrypted API access
- ✅ HTTP/2 support for improved API performance
- ✅ IPv6 support for modern networks
- ✅ Support for multicast deployment (RFC 2090)
- ❌ Web UI for management
- ❌ Image compression and deduplication
- ❌ Support for Linux deployment
- ❌ Automated driver injection
- ❌ Post-deployment configuration hooks
- ❌ Integration with Active Directory
- ❌ Metrics and monitoring

## Support

- **Issues**: [GitHub Issues](https://github.com/Wing/Snow-Owl/issues)
- **Documentation**: [Wiki](https://github.com/Wing/Snow-Owl/wiki)

---

Made with ❤️ and 🦀 Rust
