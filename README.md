# Snow-Owl 🦉

A high-performance Windows deployment tool using iPXE and WinPE, written in Rust.

Snow-Owl provides a complete PXE boot infrastructure for deploying Windows images to bare-metal machines. It features TFTP and HTTP servers for network booting, a REST API for deployment management, and automated WinPE-based deployment workflows.

## Features

- 🚀 **High Performance**: Built with Rust for speed and safety
- 🔧 **Complete PXE Stack**: TFTP and HTTP servers for network booting
- 🪟 **Windows Deployment**: Support for WIM, VHD, and VHDX image formats
- 🌐 **REST API**: Full API for automation and integration
- 📊 **Deployment Tracking**: SQLite database for tracking machines and deployments
- 🔄 **Dynamic Boot Menus**: iPXE-based boot menus generated on-the-fly
- 🛡️ **Security**: Safe Rust code with built-in protections against common vulnerabilities

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Snow-Owl Server                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ TFTP Server  │  │ HTTP Server  │  │   Database   │     │
│  │   (Port 69)  │  │  (Port 8080) │  │   (SQLite)   │     │
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

2. Edit the configuration file at `/etc/snow-owl/config.toml`:

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
database_path = "/var/lib/snow-owl/snow-owl.db"
```

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

4. Customize WinPE with deployment scripts:

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

The HTTP server exposes a REST API on port 8080 (configurable).

#### List Images

```bash
curl http://192.168.100.1:8080/api/images
```

#### Create a Deployment

```bash
curl -X POST http://192.168.100.1:8080/api/deployments \
    -H "Content-Type: application/json" \
    -d '{
        "machine_id": "uuid-of-machine",
        "image_id": "uuid-of-image"
    }'
```

#### Get Boot Menu

```bash
# Main boot menu
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
├── images/                  # Windows images
│   ├── server2022.wim
│   └── win10.vhdx
└── snow-owl.db             # SQLite database
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
│   ├── snow-owl-db/       # Database layer (SQLite)
│   ├── snow-owl-tftp/     # TFTP server implementation
│   └── snow-owl-http/     # HTTP server and REST API
├── scripts/
│   ├── winpe/             # WinPE deployment scripts
│   └── customize-winpe.sh # WinPE customization helper
└── Cargo.toml             # Workspace configuration
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
- TFTP has built-in path traversal protection
- Database uses SQLite with parameterized queries
- No authentication is implemented - deploy on trusted networks only
- Consider using VLANs to isolate deployment network

## Roadmap

- [ ] Add authentication and authorization
- [ ] Support for multicast deployment
- [ ] Web UI for management
- [ ] Image compression and deduplication
- [ ] Support for Linux deployment
- [ ] Automated driver injection
- [ ] Post-deployment configuration hooks
- [ ] Integration with Active Directory
- [ ] Metrics and monitoring

## Support

- **Issues**: [GitHub Issues](https://github.com/Wing/Snow-Owl/issues)
- **Documentation**: [Wiki](https://github.com/Wing/Snow-Owl/wiki)

---

Made with ❤️ and 🦀 Rust
