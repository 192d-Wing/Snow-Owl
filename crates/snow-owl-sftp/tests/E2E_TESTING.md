# End-to-End Testing Guide

This guide explains how to run end-to-end tests with real SFTP clients to verify compatibility and compliance.

## Overview

End-to-end tests verify that the Snow Owl SFTP server works correctly with standard SFTP clients including:

- **OpenSSH sftp** - Standard SFTP command-line client
- **OpenSSH scp** - Secure copy protocol client
- **WinSCP** - Windows SFTP/SCP client (optional)
- **FileZilla** - Cross-platform FTP/SFTP client (optional)

## Prerequisites

### Required Tools

1. **Rust Toolchain** - For building the server
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **OpenSSH Client** - Usually pre-installed on Linux/macOS
   ```bash
   # Verify installation
   which sftp ssh-keygen scp

   # On Ubuntu/Debian
   sudo apt-get install openssh-client

   # On macOS (via Homebrew)
   brew install openssh
   ```

3. **netcat** - For checking server availability
   ```bash
   # On Ubuntu/Debian
   sudo apt-get install netcat

   # On macOS
   brew install netcat
   ```

### Optional Tools

- **WinSCP** (Windows only)
- **FileZilla** (all platforms)

## Quick Start

### Automated Testing

The easiest way to run end-to-end tests is using the provided script:

```bash
cd crates/snow-owl-sftp
./tests/run_e2e_tests.sh
```

This script will:
1. Check prerequisites
2. Generate test keys (Ed25519, CNSA 2.0 compliant)
3. Start the SFTP server
4. Run all end-to-end tests
5. Clean up automatically

### Manual Testing

If you prefer to run tests manually:

#### 1. Start the Server

```bash
# Build the server
cargo build --bin snow-owl-sftp-server --release

# Generate host key (Ed25519 for CNSA 2.0)
ssh-keygen -t ed25519 -f /tmp/test_host_key -N ""

# Generate client key
ssh-keygen -t ed25519 -f ~/.ssh/test_sftp_key -N ""

# Copy public key to authorized_keys
cp ~/.ssh/test_sftp_key.pub /tmp/authorized_keys

# Start server
cargo run --bin snow-owl-sftp-server --release -- \
    --port 2222 \
    --bind-address 127.0.0.1 \
    --root-dir /tmp/sftp_root \
    --host-key-path /tmp/test_host_key \
    --authorized-keys-path /tmp/authorized_keys
```

#### 2. Run Tests

In another terminal:

```bash
cargo test --test e2e_client_tests -- --ignored --test-threads=1
```

Or run specific tests:

```bash
# Test file upload
cargo test --test e2e_client_tests test_openssh_sftp_upload -- --ignored

# Test file download
cargo test --test e2e_client_tests test_openssh_sftp_download -- --ignored

# Test large file transfer
cargo test --test e2e_client_tests test_openssh_sftp_large_file -- --ignored
```

## Manual Client Testing

### Using OpenSSH sftp

```bash
# Connect to server
sftp -i ~/.ssh/test_sftp_key -P 2222 -o "StrictHostKeyChecking=no" username@127.0.0.1

# Once connected, try commands:
sftp> put local_file.txt remote_file.txt    # Upload
sftp> get remote_file.txt local_file.txt    # Download
sftp> ls                                     # List files
sftp> mkdir testdir                          # Create directory
sftp> chmod 644 file.txt                     # Change permissions
sftp> ln -s target link                      # Create symlink
sftp> bye                                    # Disconnect
```

### Using OpenSSH scp

```bash
# Upload file
scp -i ~/.ssh/test_sftp_key -P 2222 -o "StrictHostKeyChecking=no" \
    local_file.txt username@127.0.0.1:remote_file.txt

# Download file
scp -i ~/.ssh/test_sftp_key -P 2222 -o "StrictHostKeyChecking=no" \
    username@127.0.0.1:remote_file.txt local_file.txt

# Upload directory recursively
scp -r -i ~/.ssh/test_sftp_key -P 2222 -o "StrictHostKeyChecking=no" \
    local_dir/ username@127.0.0.1:remote_dir/
```

### Using WinSCP (Windows)

```cmd
# Command-line mode
winscp.com /command ^
    "open sftp://username@127.0.0.1:2222/ -privatekey=test_sftp_key.ppk" ^
    "put local_file.txt remote_file.txt" ^
    "get remote_file.txt local_file.txt" ^
    "exit"
```

### Using FileZilla

1. Open FileZilla
2. Go to Edit > Settings > SFTP
3. Add private key file
4. Connect:
   - Host: `sftp://127.0.0.1`
   - Port: `2222`
   - Protocol: SFTP
   - Logon Type: Key file
   - User: your username
   - Key file: path to private key

## Test Coverage

### Basic Operations

- ✅ File upload (PUT)
- ✅ File download (GET)
- ✅ Directory listing (READDIR)
- ✅ Directory creation (MKDIR)
- ✅ Directory removal (RMDIR)
- ✅ File deletion (REMOVE)
- ✅ File rename (RENAME)
- ✅ File stat (STAT, LSTAT, FSTAT)

### Advanced Operations

- ✅ Set file attributes (SETSTAT, FSETSTAT)
- ✅ Change permissions (chmod)
- ✅ Change ownership (chown) - Unix only
- ✅ Symbolic link creation (SYMLINK)
- ✅ Symbolic link reading (READLINK)

### Performance Tests

- ✅ Large file transfer (100MB+)
- ✅ Multiple small files
- ✅ Concurrent connections (10+ simultaneous clients)
- ✅ Sustained throughput

### Security Tests

- ✅ SSH key authentication
- ✅ Authentication failure handling
- ✅ Rate limiting enforcement
- ✅ Path traversal prevention
- ✅ Permission boundary enforcement

### Reliability Tests

- ✅ Transfer resume after interruption
- ✅ Connection timeout handling
- ✅ Graceful error recovery
- ✅ Resource cleanup on disconnect

## NIST 800-53 Compliance Testing

These tests verify compliance with NIST 800-53 controls:

| Control | Description | Test Coverage |
|---------|-------------|---------------|
| **IA-2** | Identification and Authentication | SSH key authentication tests |
| **AC-3** | Access Enforcement | File permission tests |
| **AC-7** | Unsuccessful Logon Attempts | Rate limiting tests |
| **AC-10** | Concurrent Session Control | Connection limit tests |
| **AC-12** | Session Termination | Timeout and cleanup tests |
| **SC-8** | Transmission Confidentiality | Encrypted transfer tests |
| **SI-10** | Information Input Validation | Invalid input tests |
| **SI-11** | Error Handling | Error condition tests |

## STIG Compliance Testing

These tests verify STIG requirements:

| STIG ID | Requirement | Test Coverage |
|---------|-------------|---------------|
| **V-222611** | Public key authentication | SSH key auth tests |
| **V-222578** | Login attempt limits | Rate limiting tests |
| **V-222566** | Error message handling | Error response tests |
| **V-222596** | File permission enforcement | Permission tests |
| **V-222601** | Session termination | Cleanup tests |
| **V-222648** | Audit logging | Log generation tests |

## CNSA 2.0 Compliance Testing

All tests use CNSA 2.0 compliant cryptography:

### For UNCLASSIFIED (test default)
- **Key Type**: Ed25519 (acceptable for unclassified)
- **Key Exchange**: X25519 (Curve25519)
- **Encryption**: AES-256-GCM or AES-256-CTR
- **MAC**: HMAC-SHA-512 or HMAC-SHA-256

### For SECRET/TOP SECRET (manual testing)
Generate ECDSA P-384 keys instead:

```bash
# Generate P-384 key (CNSA 2.0 required for SECRET+)
ssh-keygen -t ecdsa -b 384 -f ~/.ssh/test_p384_key -N ""

# Use with sftp
sftp -i ~/.ssh/test_p384_key -o "HostKeyAlgorithms=ecdsa-sha2-nistp384" \
     -P 2222 username@127.0.0.1
```

## Troubleshooting

### Server Won't Start

```bash
# Check if port is already in use
lsof -i :2222

# Check server logs
tail -f /var/log/snow-owl/sftp-audit.json

# Verify host key permissions (must be 0600)
ls -l /tmp/test_host_key
chmod 600 /tmp/test_host_key
```

### Authentication Fails

```bash
# Verify key permissions (must be 0600)
chmod 600 ~/.ssh/test_sftp_key

# Verify authorized_keys format
cat /tmp/authorized_keys

# Check authorized_keys permissions
chmod 600 /tmp/authorized_keys

# Enable debug mode
sftp -vvv -i ~/.ssh/test_sftp_key -P 2222 username@127.0.0.1
```

### Connection Refused

```bash
# Verify server is running
ps aux | grep snow-owl-sftp-server

# Check server is listening
netcat -zv 127.0.0.1 2222

# Check firewall rules
sudo iptables -L -n | grep 2222
```

### Tests Fail

```bash
# Run with verbose output
cargo test --test e2e_client_tests -- --ignored --nocapture

# Check server logs for errors
cat /tmp/sftp_root/../server.log

# Verify test prerequisites
./tests/run_e2e_tests.sh --check-only
```

## Performance Benchmarking

### Throughput Test

```bash
# Create 1GB test file
dd if=/dev/zero of=/tmp/1gb.bin bs=1M count=1024

# Time upload
time sftp -i ~/.ssh/test_sftp_key -P 2222 username@127.0.0.1 <<EOF
put /tmp/1gb.bin test_upload.bin
bye
EOF

# Time download
time sftp -i ~/.ssh/test_sftp_key -P 2222 username@127.0.0.1 <<EOF
get test_upload.bin /tmp/test_download.bin
bye
EOF
```

### Concurrent Connections

```bash
# Test 50 concurrent uploads
for i in {1..50}; do
    (
        sftp -i ~/.ssh/test_sftp_key -P 2222 username@127.0.0.1 <<EOF
put /tmp/test.txt test_$i.txt
bye
EOF
    ) &
done
wait
```

## Continuous Integration

### GitHub Actions Example

```yaml
name: E2E Tests

on: [push, pull_request]

jobs:
  e2e-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y openssh-client netcat

      - name: Run E2E tests
        run: |
          cd crates/snow-owl-sftp
          ./tests/run_e2e_tests.sh
```

## Contributing

When adding new E2E tests:

1. Mark tests with `#[ignore]` so they don't run in unit test CI
2. Add documentation for what the test verifies
3. Include NIST 800-53 and STIG references
4. Check for client availability before running
5. Clean up resources properly
6. Update this documentation

## References

- [NIST 800-53 Rev. 5](https://csrc.nist.gov/publications/detail/sp/800-53/rev-5/final)
- [DISA STIG](https://public.cyber.mil/stigs/)
- [NSA CNSA 2.0](https://media.defense.gov/2022/Sep/07/2003071834/-1/-1/0/CSA_CNSA_2.0_ALGORITHMS_.PDF)
- [OpenSSH Manual](https://www.openssh.com/manual.html)
- [SFTP Protocol Draft](https://datatracker.ietf.org/doc/html/draft-ietf-secsh-filexfer-02)
