# Security Policy

## Compliance Framework

Snow Owl SFTP is developed with strict adherence to security standards:

- **NIST 800-53 Rev 5**: Security and Privacy Controls
- **Application Security STIG**: DoD Application Security Requirements
- **OWASP Top 10**: Web Application Security Risks
- **CWE Top 25**: Most Dangerous Software Weaknesses

## NIST 800-53 Controls Implementation

### Access Control (AC)

| Control | Implementation | File Reference |
|---------|----------------|----------------|
| AC-2 | Account Management via authorized_keys | server.rs |
| AC-3 | Path traversal protection, access enforcement | server.rs:resolve_path |
| AC-7 | Authentication attempt limiting (planned) | Phase 1.1 |
| AC-11 | Session timeouts | config.rs:timeout |
| AC-12 | Automatic session termination | server.rs |

### Audit and Accountability (AU)

| Control | Implementation | File Reference |
|---------|----------------|----------------|
| AU-2 | File operation auditing | server.rs |
| AU-3 | Detailed audit records with user, timestamp | server.rs |
| AU-8 | UTC timestamps for all operations | server.rs |
| AU-9 | Protected audit trail (planned) | Phase 2.3 |
| AU-12 | Audit event generation | server.rs |

### Identification and Authentication (IA)

| Control | Implementation | File Reference |
|---------|----------------|----------------|
| IA-2 | SSH public key authentication | server.rs:auth_publickey |
| IA-3 | Device identification via SSH | russh library |
| IA-5 | Authenticator management | server.rs |

### System and Communications Protection (SC)

| Control | Implementation | File Reference |
|---------|----------------|----------------|
| SC-8 | SSH encryption for all data | russh library |
| SC-13 | FIPS 140-2 validated crypto (via russh) | russh library |
| SC-23 | Session token protection | russh library |

### System and Information Integrity (SI)

| Control | Implementation | File Reference |
|---------|----------------|----------------|
| SI-10 | Input validation on all paths/data | protocol.rs, server.rs |
| SI-11 | Secure error handling without leaks | error.rs |

## Application Security STIG Compliance

### Implemented Controls

| STIG ID | Requirement | Implementation |
|---------|-------------|----------------|
| V-222396 | Input validation | protocol.rs:codec, server.rs:resolve_path |
| V-222566 | Secure error messages | error.rs |
| V-222577 | Cryptographic protection | russh library (AES, ChaCha20) |
| V-222596 | Authorization enforcement | server.rs:resolve_path |
| V-222601 | Session termination | config.rs:timeout, server.rs |
| V-222602 | Session ID protection | russh library |

### Planned Controls (Roadmap Phase 1-2)

| STIG ID | Requirement | Target Phase |
|---------|-------------|--------------|
| V-222575 | Audit information protection | Phase 2.3 |
| V-222576 | Audit tool protection | Phase 2.3 |
| V-222578 | Replay-resistant auth | Phase 2.1 |
| V-222597 | Separation of duties | Phase 2.4 |
| V-222611 | Certificate validation | Phase 2.1 |

## Security Features

### Path Traversal Protection (CWE-22)

**Implementation**: `server.rs:resolve_path()`

```rust
// NIST 800-53: AC-3 (Access Enforcement)
// STIG: V-222596
if !resolved.starts_with(&self.config.root_dir) {
    return Err(Error::PermissionDenied("Path traversal attempt"));
}
```

Prevents:
- `../../../etc/passwd`
- Symlink attacks outside root
- Absolute path escapes

### Input Validation (CWE-20)

**Implementation**: `protocol.rs:codec`

All inputs are validated:
- String length limits
- UTF-8 validation
- Buffer bounds checking
- Packet size limits (RFC 4254: min 32768 bytes)

### Secure Error Handling (CWE-209)

**Implementation**: `error.rs`

- No stack traces to clients
- No sensitive path information leaked
- Generic error messages for security failures
- Detailed logging server-side only

### Cryptographic Protection

**Implementation**: Via `russh` library

Supported algorithms:
- **Encryption**: AES-128/192/256-CTR, ChaCha20-Poly1305
- **MAC**: HMAC-SHA2-256, HMAC-SHA2-512
- **Key Exchange**: Curve25519, ECDH SHA2 NISTP256/384/521
- **Host Keys**: Ed25519, RSA, ECDSA

### Session Management

**Implementation**: `config.rs`, `server.rs`

- Configurable session timeout (default: 300s)
- Automatic cleanup of stale sessions
- Secure session token handling via SSH
- No predictable session identifiers

## Vulnerability Reporting

### Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

### Reporting a Vulnerability

**DO NOT** open public issues for security vulnerabilities.

Instead:

1. Email: security@snow-owl.dev (if available)
2. Use GitHub Security Advisories (preferred)
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Impact assessment
   - Suggested fix (if any)

### Response Timeline

- **Initial Response**: Within 48 hours
- **Triage**: Within 1 week
- **Fix Development**: Based on severity
  - Critical: 1-7 days
  - High: 1-2 weeks
  - Medium: 2-4 weeks
  - Low: Next release cycle

### Disclosure Policy

- **Coordinated Disclosure**: 90 days from report
- **Early Disclosure**: If actively exploited
- **Credit**: Reporter credited in CHANGELOG (if desired)

## Known Security Limitations

### Current Version (0.1.0)

1. **Authentication**
   - Accepts all public keys (development mode)
   - No authorized_keys verification yet
   - ⚠️ **DO NOT USE IN PRODUCTION** until Phase 1.1 complete

2. **Rate Limiting**
   - No authentication attempt limiting
   - No connection rate limiting
   - Planned for Phase 1.1

3. **Audit Logging**
   - Basic logging only
   - No tamper-proof audit trail
   - Enhanced logging in Phase 2.3

4. **Protocol Version**
   - SFTP v3 only
   - No v4+ features (ACLs, extended attributes)
   - Planned for Phase 4

### Mitigations

For development/testing only:
- Deploy behind firewall
- Use network segmentation
- Enable verbose logging
- Monitor all connections
- Restrict to trusted networks

## Security Checklist for Deployment

Before production deployment:

### Phase 1 Requirements (Minimum)
- [ ] Authorized keys verification implemented
- [ ] Authentication rate limiting enabled
- [ ] Session timeout configured (≤300s)
- [ ] Root directory properly configured
- [ ] Host key generated securely (not temporary)
- [ ] Firewall rules configured
- [ ] Audit logging enabled
- [ ] Regular log monitoring setup

### Phase 2 Requirements (Recommended)
- [ ] Advanced file permissions implemented
- [ ] User quotas configured
- [ ] Bandwidth limiting enabled
- [ ] Enhanced audit logging
- [ ] Monitoring/alerting configured
- [ ] Regular security updates scheduled

### Phase 5 Requirements (Enterprise)
- [ ] High availability configured
- [ ] Two-factor authentication enabled
- [ ] Compliance reporting active
- [ ] Intrusion detection integrated
- [ ] Regular penetration testing

## Secure Configuration

### Minimum Security Configuration

```toml
# config.toml - Minimum security settings

bind_address = "0.0.0.0"  # Restrict to specific IP in production
port = 2222                # Non-privileged port
root_dir = "/srv/sftp"     # Isolated directory
timeout = 300              # 5 minute timeout
max_connections = 100      # Prevent resource exhaustion
max_packet_size = 32768    # RFC 4254 minimum
window_size = 2097152      # 2MB default

# Use strong host key
host_key_path = "/etc/snow-owl/ssh_host_ed25519_key"

# Verify authorized keys
authorized_keys_path = "/etc/snow-owl/authorized_keys"
```

### Host Key Generation

```bash
# Generate Ed25519 key (recommended - best security/performance)
ssh-keygen -t ed25519 -f /etc/snow-owl/ssh_host_ed25519_key -N ""

# Generate RSA key (compatibility)
ssh-keygen -t rsa -b 4096 -f /etc/snow-owl/ssh_host_rsa_key -N ""

# Set proper permissions
chmod 600 /etc/snow-owl/ssh_host_*_key
chmod 644 /etc/snow-owl/ssh_host_*_key.pub
```

### Filesystem Permissions

```bash
# SFTP root directory
mkdir -p /srv/sftp
chown sftp-user:sftp-group /srv/sftp
chmod 755 /srv/sftp

# Prevent execution
mount -o noexec /srv/sftp  # If separate partition
```

## Security Hardening

### Operating System Level

```bash
# Firewall rules (iptables example)
iptables -A INPUT -p tcp --dport 2222 -s TRUSTED_IP -j ACCEPT
iptables -A INPUT -p tcp --dport 2222 -j DROP

# SELinux/AppArmor profile (recommended)
# Create custom profile for snow-owl-sftp

# Resource limits
ulimit -n 65535  # File descriptors
ulimit -u 512    # Max processes
```

### Network Level

- Deploy behind reverse proxy/load balancer
- Use VPN for remote access
- Implement network segmentation
- Enable DDoS protection
- Use intrusion detection (Snort, Suricata)

### Application Level

- Enable verbose logging
- Monitor for suspicious patterns
- Implement alert thresholds
- Regular log analysis
- Automated security scanning

## Cryptographic Standards

### Algorithms (via russh)

**Recommended Configuration**:
- Key Exchange: Curve25519
- Encryption: ChaCha20-Poly1305, AES-256-GCM
- MAC: (implicit in AEAD modes)
- Host Key: Ed25519

**Avoid** (weak algorithms):
- DES, 3DES
- RC4, Blowfish
- MD5, SHA-1
- RSA < 2048 bits

## Compliance Verification

### Self-Assessment

Run security checks:

```bash
# 1. Code security audit
cargo audit

# 2. Dependency vulnerabilities
cargo deny check

# 3. Static analysis
cargo clippy -- -D warnings

# 4. STIG compliance check
./verify.sh
```

### Third-Party Assessment

Consider:
- Annual penetration testing
- Security code review
- STIG compliance scan
- Vulnerability assessment

## References

- [NIST 800-53 Rev 5](https://csrc.nist.gov/publications/detail/sp/800-53/rev-5/final)
- [Application Security STIG](https://public.cyber.mil/stigs/)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [CWE Top 25](https://cwe.mitre.org/top25/)
- [RFC 4251-4254](https://datatracker.ietf.org/doc/html/rfc4251)

---

**Last Updated**: 2026-01-19
**Security Contact**: security@snow-owl.dev
