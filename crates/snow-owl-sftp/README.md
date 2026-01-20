# Snow Owl SFTP

RFC-compliant SFTP (SSH File Transfer Protocol) server and client implementation in Rust.

## Overview

This crate provides a full implementation of the SFTP protocol built on top of the SSH protocol suite. It follows the specifications defined in:

- **RFC 4251**: SSH Protocol Architecture
- **RFC 4252**: SSH Authentication Protocol
- **RFC 4253**: SSH Transport Layer Protocol
- **RFC 4254**: SSH Connection Protocol
- **draft-ietf-secsh-filexfer-02**: SSH File Transfer Protocol (SFTP)

## Features

- ✅ **RFC Compliant**: Full implementation of SFTP protocol specification
- ✅ **Async/Await**: Built with Tokio for high-performance async I/O
- ✅ **SSH Authentication**: Support for public key authentication
- ✅ **File Operations**: Read, write, delete, rename files
- ✅ **Directory Operations**: List, create, remove directories
- ✅ **File Attributes**: Full support for file metadata (size, permissions, timestamps)
- ✅ **Security**: Path traversal protection and proper permission handling
- 🚧 **Client Implementation**: Basic client structure (work in progress)

## Protocol Support

### Implemented SFTP Messages

- `SSH_FXP_INIT` / `SSH_FXP_VERSION` - Protocol initialization
- `SSH_FXP_OPEN` - Open file
- `SSH_FXP_CLOSE` - Close file handle
- `SSH_FXP_READ` - Read from file
- `SSH_FXP_WRITE` - Write to file
- `SSH_FXP_STAT` / `SSH_FXP_LSTAT` - Get file attributes
- `SSH_FXP_FSTAT` - Get attributes by handle
- `SSH_FXP_OPENDIR` - Open directory
- `SSH_FXP_READDIR` - Read directory entries
- `SSH_FXP_REMOVE` - Remove file
- `SSH_FXP_MKDIR` - Create directory
- `SSH_FXP_RMDIR` - Remove directory
- `SSH_FXP_REALPATH` - Resolve path
- `SSH_FXP_RENAME` - Rename file/directory
- `SSH_FXP_STATUS` - Status response
- `SSH_FXP_HANDLE` - File handle response
- `SSH_FXP_DATA` - Data response
- `SSH_FXP_NAME` - Name response
- `SSH_FXP_ATTRS` - Attributes response

### SFTP Protocol Version

This implementation supports **SFTP version 3**, which is the most widely supported version and provides all essential file transfer operations.

## Usage

### Server

Run the SFTP server:

```bash
# Using default settings (port 2222, /tmp/sftp root)
cargo run --bin snow-owl-sftp-server

# With custom configuration
cargo run --bin snow-owl-sftp-server -- \
  --bind 0.0.0.0 \
  --port 2222 \
  --root /srv/sftp \
  --verbose

# Using a configuration file
cargo run --bin snow-owl-sftp-server -- --config config.toml
```

### Configuration File

Create a `config.toml`:

```toml
bind_address = "0.0.0.0"
port = 2222
root_dir = "/srv/sftp"
host_key_path = "/etc/ssh/ssh_host_rsa_key"
authorized_keys_path = "~/.ssh/authorized_keys"
max_connections = 100
timeout = 300
verbose = true
max_packet_size = 32768
window_size = 2097152
```

### Client (Work in Progress)

```bash
# List directory
cargo run --bin snow-owl-sftp-client -- ls /

# Upload file
cargo run --bin snow-owl-sftp-client -- put local.txt /remote.txt

# Download file
cargo run --bin snow-owl-sftp-client -- get /remote.txt local.txt

# Create directory
cargo run --bin snow-owl-sftp-client -- mkdir /newdir

# Remove file
cargo run --bin snow-owl-sftp-client -- rm /file.txt
```

### Library Usage

```rust
use snow_owl_sftp::{Config, Server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    let server = Server::new(config).await?;
    server.run().await?;
    Ok(())
}
```

## Architecture

### Protocol Layer

The `protocol` module implements the core SFTP message types and encoding/decoding:

- Message type definitions (INIT, OPEN, READ, WRITE, etc.)
- Status codes (OK, EOF, NO_SUCH_FILE, etc.)
- File attributes structure
- Open flags for file operations
- Codec helpers for SFTP string/bytes encoding

### Server Implementation

The server uses `russh` for SSH protocol handling and implements:

- SSH session management
- SFTP subsystem handling
- File handle management
- Path resolution with security checks
- Async file I/O operations
- RFC-compliant message processing

### Security Features

- **Path Traversal Protection**: All paths are validated to stay within the configured root directory
- **SSH Authentication**: Public key authentication support
- **Configurable Timeouts**: Connection timeout handling
- **Flow Control**: Proper window size and packet size limits per RFC 4254

## RFC Compliance

This implementation follows the SFTP specification closely:

1. **Packet Format**: All messages use the correct format with message type and request ID
2. **Status Codes**: Proper status codes returned for all operations
3. **File Attributes**: Complete attribute support with flags
4. **String Encoding**: UTF-8 strings with length prefix
5. **Error Handling**: Appropriate error codes for all failure cases
6. **Flow Control**: Respects SSH channel window sizes

### Minimum Requirements (RFC 4254)

- ✅ Maximum packet size: 32768 bytes minimum (configurable)
- ✅ Window size: 2MB default (configurable)
- ✅ Proper channel management
- ✅ SFTP subsystem activation

## Development Status

### Completed ✅

- Core protocol implementation
- Server with full SFTP support
- File operations (read, write, delete, rename)
- Directory operations (list, create, remove)
- RFC-compliant message handling
- Security features (path validation, authentication)

### In Progress 🚧

- Full client implementation
- Extended attributes support
- Symbolic link operations
- Advanced authentication methods

### Planned 📋

- SFTP protocol version 4+ extensions
- Performance optimizations
- Comprehensive test suite
- Integration tests
- Benchmarks

## Testing

Connect to the server using standard SFTP clients:

```bash
# Using sftp command-line client
sftp -P 2222 user@localhost

# Using FileZilla or other GUI clients
# Host: localhost
# Port: 2222
# Protocol: SFTP
```

## Contributing

Contributions are welcome! This crate aims to be a fully RFC-compliant SFTP implementation.

## License

MIT OR Apache-2.0

## References

- [RFC 4251 - SSH Protocol Architecture](https://tools.ietf.org/html/rfc4251)
- [RFC 4252 - SSH Authentication Protocol](https://tools.ietf.org/html/rfc4252)
- [RFC 4253 - SSH Transport Layer Protocol](https://tools.ietf.org/html/rfc4253)
- [RFC 4254 - SSH Connection Protocol](https://tools.ietf.org/html/rfc4254)
- [draft-ietf-secsh-filexfer-02 - SFTP Protocol](https://datatracker.ietf.org/doc/html/draft-ietf-secsh-filexfer-02)
