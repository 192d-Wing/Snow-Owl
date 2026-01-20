# Quick Start Guide

Get up and running with Snow Owl SFTP in minutes.

## Prerequisites

- Rust 1.70 or later
- Linux, macOS, or Windows

## Installation

### Build from Source

```bash
# Clone the repository
git clone https://github.com/Wing/Snow-Owl.git
cd Snow-Owl

# Build the SFTP crate
cargo build --release -p snow-owl-sftp

# Binaries will be in target/release/
```

## Running the Server

### Quick Start (Development)

```bash
# Create a root directory for SFTP
mkdir -p /tmp/sftp

# Run the server with default settings
cargo run --bin snow-owl-sftp-server -- \
  --root /tmp/sftp \
  --verbose
```

The server will:
- Listen on `0.0.0.0:2222`
- Use `/tmp/sftp` as the root directory
- Generate a temporary host key (since we don't have one)
- Accept all public key authentication (for testing)

### Production Setup

1. **Generate or use existing SSH host key:**

```bash
# Use existing system key
sudo cp /etc/ssh/ssh_host_rsa_key /etc/snow-owl/host_key
sudo chmod 600 /etc/snow-owl/host_key
```

2. **Create configuration file:**

```bash
cp crates/snow-owl-sftp/config.example.toml /etc/snow-owl/sftp.toml
```

Edit `/etc/snow-owl/sftp.toml`:
```toml
bind_address = "0.0.0.0"
port = 2222
root_dir = "/srv/sftp"
host_key_path = "/etc/snow-owl/host_key"
max_connections = 100
timeout = 300
```

3. **Run the server:**

```bash
sudo mkdir -p /srv/sftp
sudo chown sftp:sftp /srv/sftp

cargo run --release --bin snow-owl-sftp-server -- \
  --config /etc/snow-owl/sftp.toml
```

## Connecting to the Server

### Using OpenSSH sftp client

```bash
# Connect to the server
sftp -P 2222 user@localhost

# Common commands
sftp> ls
sftp> put localfile.txt
sftp> get remotefile.txt
sftp> mkdir newdir
sftp> cd newdir
sftp> rm file.txt
sftp> bye
```

### Using FileZilla

1. Open FileZilla
2. Go to File → Site Manager
3. Click "New Site"
4. Configure:
   - Protocol: SFTP - SSH File Transfer Protocol
   - Host: localhost (or your server IP)
   - Port: 2222
   - Logon Type: Key file or Normal
   - User: your username
5. Click "Connect"

### Using the Snow Owl Client (when implemented)

```bash
# List directory
cargo run --bin snow-owl-sftp-client -- \
  --host localhost --port 2222 \
  ls /

# Upload file
cargo run --bin snow-owl-sftp-client -- \
  --host localhost --port 2222 \
  put local.txt /remote.txt

# Download file
cargo run --bin snow-owl-sftp-client -- \
  --host localhost --port 2222 \
  get /remote.txt local.txt
```

## Common Use Cases

### File Server

```bash
# Run server on standard SSH port (requires root)
sudo cargo run --release --bin snow-owl-sftp-server -- \
  --port 22 \
  --root /srv/files
```

### Development File Transfer

```bash
# Quick server for local development
cargo run --bin snow-owl-sftp-server -- \
  --root ./shared \
  --verbose
```

### Automated Backups

Configure your backup tool to use SFTP with:
- Host: your-server
- Port: 2222
- Protocol: SFTP
- Authentication: SSH key

## Testing the Installation

### 1. Start the server

```bash
cargo run --bin snow-owl-sftp-server -- \
  --root /tmp/sftp_test \
  --verbose
```

### 2. In another terminal, create a test file

```bash
echo "Hello, SFTP!" > /tmp/test.txt
```

### 3. Connect and upload

```bash
sftp -P 2222 user@localhost
sftp> put /tmp/test.txt
sftp> ls
sftp> get test.txt /tmp/downloaded.txt
sftp> bye
```

### 4. Verify

```bash
cat /tmp/downloaded.txt
# Should output: Hello, SFTP!
```

## Troubleshooting

### Connection Refused

```bash
# Check if server is running
netstat -ln | grep 2222

# Check firewall rules
sudo ufw allow 2222/tcp
```

### Permission Denied

```bash
# Ensure root directory exists and is writable
mkdir -p /tmp/sftp
chmod 755 /tmp/sftp

# Check server logs for details
cargo run --bin snow-owl-sftp-server -- --verbose
```

### Host Key Issues

```bash
# For testing, let server generate a temporary key
# No --host-key argument needed

# For production, use a real key:
ssh-keygen -t rsa -b 4096 -f /etc/snow-owl/host_key -N ""
```

### Authentication Failures

Current implementation accepts all public key authentication for testing.

For production:
1. Implement proper authorized_keys verification
2. Configure SSH agent forwarding if needed
3. Use proper key-based authentication

## Next Steps

- Read [README.md](README.md) for detailed features
- Check [RFC_COMPLIANCE.md](RFC_COMPLIANCE.md) for protocol details
- Review [config.example.toml](config.example.toml) for all options
- Run tests: `cargo test -p snow-owl-sftp`

## Security Notes

⚠️ **Important for Production:**

1. **Do not use default settings in production**
2. **Use proper SSH host keys** (not auto-generated)
3. **Implement authorized_keys verification**
4. **Use firewall rules** to restrict access
5. **Enable logging and monitoring**
6. **Use strong authentication** (key-based, not password)
7. **Restrict root directory** permissions appropriately
8. **Keep software updated**

## Getting Help

- Check the logs with `--verbose` flag
- Review error messages in the console
- Check file permissions
- Verify network connectivity
- Read the RFC compliance documentation

## Example Session

```bash
# Terminal 1: Start server
$ cargo run --bin snow-owl-sftp-server -- --root /tmp/sftp --verbose
INFO Starting SFTP server on 0.0.0.0:2222
INFO Root Directory: /tmp/sftp

# Terminal 2: Connect and use
$ sftp -P 2222 user@localhost
Connected to localhost.
sftp> pwd
Remote working directory: /
sftp> mkdir uploads
sftp> cd uploads
sftp> put document.pdf
Uploading document.pdf to /uploads/document.pdf
document.pdf                    100%   1024KB   1.0MB/s   00:01
sftp> ls -l
-rw-r--r--    1 1000     1000      1048576 Jan 19 21:00 document.pdf
sftp> bye
```

That's it! You now have a working RFC-compliant SFTP server.
