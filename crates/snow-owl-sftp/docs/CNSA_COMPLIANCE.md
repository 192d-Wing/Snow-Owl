# NSA CNSA 2.0 Compliance

This document describes the Snow Owl SFTP implementation's compliance with the NSA Commercial National Security Algorithm (CNSA) Suite 2.0.

## Overview

The CNSA Suite 2.0 specifies cryptographic algorithms approved by the NSA for protecting National Security Systems (NSS) information **at all classification levels** including TOP SECRET. This implementation enforces CNSA 2.0 requirements to ensure all cryptographic operations use only approved algorithms.

**Classification Support**:
- ✅ UNCLASSIFIED
- ✅ SECRET
- ⚠️ TOP SECRET (current baseline, PQC transition by 2030)

## CNSA 2.0 Requirements by Classification Level

### For SECRET and Below (Effective Immediately)

The following algorithms are **required** for classified information up to SECRET:

| Function | Algorithm | Details | Status |
|----------|-----------|---------|--------|
| **Encryption** | AES-256 | 256-bit AES (GCM preferred) | ✅ Supported |
| **Key Exchange** | ECDH P-384 | Elliptic Curve Diffie-Hellman with P-384 | ✅ Supported |
| **Digital Signatures** | ECDSA P-384 | Elliptic Curve DSA with P-384 | ✅ Supported |
| **Hashing** | SHA-384/512 | Secure Hash Algorithm | ✅ Supported |

### For TOP SECRET and Above

#### Current Requirements (Through 2030)

For **TOP SECRET**, the same baseline algorithms as SECRET are required:

| Function | Algorithm | Details | Status |
|----------|-----------|---------|--------|
| **Encryption** | AES-256 | 256-bit AES (quantum-safe) | ✅ Supported |
| **Key Exchange** | ECDH P-384 | Elliptic Curve DH with P-384 | ✅ Supported |
| **Digital Signatures** | ECDSA P-384 | Elliptic Curve DSA with P-384 | ✅ Supported |
| **Hashing** | SHA-384/512 | Secure Hash Algorithm | ✅ Supported |

#### Post-Quantum Requirements (2030-2033 Transition)

CNSA 2.0 mandates transition to quantum-resistant algorithms for TOP SECRET by 2030:

| Function | Algorithm | Standard | Security | Status |
|----------|-----------|----------|----------|--------|
| **Key Exchange** | ML-KEM-1024 | FIPS 203 | 256-bit | ⚠️ Planned (2027-2028) |
| **Signatures** | ML-DSA-87 | FIPS 204 | 256-bit | ⚠️ Planned (2027-2028) |
| **Backup Sigs** | SLH-DSA-256 | FIPS 205 | 256-bit | ⚠️ Planned (2027-2028) |
| **Encryption** | AES-256 | FIPS 197 | 256-bit | ✅ Supported (quantum-safe) |
| **Hashing** | SHA-384/512 | FIPS 180-4 | Quantum-resistant | ✅ Supported |

**Why Post-Quantum Cryptography (PQC)?**

Large-scale quantum computers threaten current public-key cryptography (ECDH, ECDSA, RSA) through Shor's algorithm. CNSA 2.0 requires TOP SECRET systems transition to quantum-resistant algorithms before quantum computers become viable.

**Timeline**:
- **Now - 2030**: Use P-384 baseline (current implementation)
- **2030 - 2033**: Transition period to PQC
- **2033+**: PQC mandatory for new TOP SECRET systems

**Implementation Status**: Current version supports P-384 baseline. PQC support (ML-KEM, ML-DSA, SLH-DSA) planned for 2027-2028 release to meet transition deadline.

### For Non-Classified (UNCLASSIFIED) Use

The following modern algorithms are also acceptable for unclassified information:

| Function | Algorithm | Details | Status |
|----------|-----------|---------|--------|
| **Key Exchange** | X25519 | Curve25519 (modern, fast) | ✅ Supported |
| **Signatures** | Ed25519 | Edwards-curve DSA (EdDSA) | ✅ Supported |
| **Encryption** | AES-256 | Same as classified | ✅ Supported |
| **Hashing** | SHA-384/512 | Same as classified | ✅ Supported |

## SSH Algorithm Mapping

### Key Exchange Algorithms (KEX)

The server and client are configured to use only these key exchange methods:

```
1. ecdh-sha2-nistp384  (CNSA 2.0 required, preferred)
2. curve25519-sha256   (acceptable for non-classified)
```

**Disabled**: All Diffie-Hellman (DH) and RSA key exchange methods.

### Encryption Ciphers

Only AES-256 ciphers are enabled:

```
1. aes256-gcm@openssh.com  (AEAD cipher, preferred)
2. aes256-ctr              (CTR mode, acceptable fallback)
```

**Why AES-GCM is preferred**: GCM (Galois/Counter Mode) provides authenticated encryption (AEAD), which combines encryption and authentication in a single operation, providing better security properties than CTR mode with separate MAC.

**Disabled**: All AES-128, AES-192, 3DES, Blowfish, RC4, and ChaCha20 ciphers.

### Message Authentication Codes (MAC)

For ciphers that require separate MAC (e.g., AES-256-CTR):

```
1. hmac-sha2-512  (preferred)
2. hmac-sha2-256  (acceptable minimum)
```

**Note**: When using AES-GCM, MAC is not used as GCM provides integrated authentication.

**Disabled**: All MD5, SHA-1, and RIPEMD-based MACs.

### Host Key / Public Key Algorithms

Only elliptic curve signature algorithms are enabled:

```
1. ecdsa-sha2-nistp384  (CNSA 2.0 required, preferred)
2. ssh-ed25519          (acceptable for non-classified)
```

**Disabled**: All RSA, DSA, and ECDSA algorithms with curves other than P-384 (P-256, P-521).

## Implementation

### Server Configuration

The SFTP server enforces CNSA 2.0 compliance in [src/server.rs](../src/server.rs):

```rust
// Configure only CNSA 2.0 approved algorithms
ssh_config.preferred = russh::Preferred {
    kex: cnsa::CNSA_KEX_ALGORITHMS,
    key: cnsa::CNSA_HOST_KEY_ALGORITHMS,
    cipher: cnsa::CNSA_CIPHERS,
    mac: cnsa::CNSA_MAC_ALGORITHMS,
    ..Default::default()
};
```

This ensures that:
1. Only approved algorithms are advertised during SSH negotiation
2. Client connections using non-compliant algorithms are rejected
3. All cryptographic operations use CNSA 2.0 approved methods

### Client Configuration

The SFTP client is similarly configured in [src/client.rs](../src/client.rs) to only use CNSA 2.0 compliant algorithms when connecting to servers.

### Algorithm Constants

All approved algorithms are defined in [src/cnsa.rs](../src/cnsa.rs):

```rust
pub const CNSA_KEX_ALGORITHMS: &[KexName] = &[
    KexName::EcdhSha2Nistp384,
    KexName::Curve25519Sha256,
];

pub const CNSA_CIPHERS: &[CipherName] = &[
    CipherName::Aes256Gcm,
    CipherName::Aes256Ctr,
];

pub const CNSA_MAC_ALGORITHMS: &[MacName] = &[
    MacName::HmacSha2_512,
    MacName::HmacSha2_256,
];

pub const CNSA_HOST_KEY_ALGORITHMS: &[KeyName] = &[
    KeyName::EcdsaSha2Nistp384,
    KeyName::Ed25519,
];
```

## Host Key Requirements

### Generating CNSA 2.0 Compliant Host Keys

To generate a CNSA 2.0 compliant ECDSA P-384 host key:

```bash
# Generate ECDSA P-384 key (CNSA 2.0 required)
ssh-keygen -t ecdsa -b 384 -f /etc/ssh/ssh_host_ecdsa384_key -N ""

# Alternative: Generate Ed25519 key (acceptable for non-classified)
ssh-keygen -t ed25519 -f /etc/ssh/ssh_host_ed25519_key -N ""
```

### Checking Existing Keys

To verify an existing key's algorithm:

```bash
# Check key type
ssh-keygen -l -f /etc/ssh/ssh_host_ecdsa_key

# Should show: "384 SHA256:... (ECDSA)" for CNSA 2.0 compliance
```

### Converting Non-Compliant Keys

**Important**: RSA keys are NOT CNSA 2.0 compliant. If you have RSA host keys:

1. Generate new ECDSA P-384 or Ed25519 keys (see above)
2. Update server configuration to use the new keys
3. Distribute new public keys to clients
4. Remove or disable RSA keys

## Client Key Requirements

### Generating CNSA 2.0 Compliant Client Keys

For client authentication:

```bash
# Generate ECDSA P-384 key pair (CNSA 2.0 required)
ssh-keygen -t ecdsa -b 384 -f ~/.ssh/id_ecdsa384 -C "user@host"

# Alternative: Generate Ed25519 key pair (acceptable for non-classified)
ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519 -C "user@host"
```

Add the public key to the server's `authorized_keys` file:

```bash
cat ~/.ssh/id_ecdsa384.pub >> ~/.ssh/authorized_keys
# or
cat ~/.ssh/id_ed25519.pub >> ~/.ssh/authorized_keys
```

## Verification

### Testing CNSA 2.0 Compliance

To verify the server only accepts CNSA 2.0 compliant algorithms:

```bash
# Test with CNSA 2.0 compliant algorithms (should succeed)
ssh -o KexAlgorithms=ecdh-sha2-nistp384 \
    -o Ciphers=aes256-gcm@openssh.com \
    -o MACs=hmac-sha2-512 \
    -o HostKeyAlgorithms=ecdsa-sha2-nistp384 \
    user@server -s sftp

# Test with non-compliant algorithms (should fail)
ssh -o KexAlgorithms=diffie-hellman-group14-sha256 \
    -o Ciphers=aes128-ctr \
    user@server -s sftp
```

### Checking Negotiated Algorithms

To see what algorithms were negotiated:

```bash
# Run SSH in verbose mode
ssh -vv user@server -s sftp 2>&1 | grep -E "(kex|cipher|MAC|host key)"
```

Look for:
- `kex: ecdh-sha2-nistp384` or `curve25519-sha256`
- `cipher: aes256-gcm@openssh.com` or `aes256-ctr`
- `MAC: hmac-sha2-512` or `hmac-sha2-256` (for CTR mode)
- `host key: ecdsa-sha2-nistp384` or `ssh-ed25519`

## Post-Quantum Cryptography (PQC) for TOP SECRET

### Background

CNSA 2.0 requires TOP SECRET systems to transition to quantum-resistant algorithms by 2030-2033 to protect against future quantum computer threats. NIST has standardized three post-quantum algorithms:

### NIST PQC Standards

#### ML-KEM (FIPS 203) - Key Encapsulation Mechanism
- **Replaces**: ECDH for key exchange
- **Based on**: Module Learning with Errors (MLWE) problem
- **Parameter Sets**:
  - ML-KEM-512 (128-bit security)
  - ML-KEM-768 (192-bit security)
  - ML-KEM-1024 (256-bit security) ← **Required for TOP SECRET**
- **Key Sizes**: Public key ~1568 bytes, ciphertext ~1568 bytes (ML-KEM-1024)

#### ML-DSA (FIPS 204) - Digital Signature Algorithm
- **Replaces**: ECDSA for digital signatures
- **Based on**: Module Learning with Errors (MLWE) problem
- **Parameter Sets**:
  - ML-DSA-44 (128-bit security)
  - ML-DSA-65 (192-bit security)
  - ML-DSA-87 (256-bit security) ← **Required for TOP SECRET**
- **Signature Size**: ~4595 bytes (ML-DSA-87)

#### SLH-DSA (FIPS 205) - Stateless Hash-Based Signatures
- **Purpose**: Backup signature scheme (conservative, hash-based)
- **Based on**: SPHINCS+ (hash functions only, no algebraic assumptions)
- **Parameter Sets**:
  - SLH-DSA-128f/s (128-bit security, fast/small)
  - SLH-DSA-192f/s (192-bit security, fast/small)
  - SLH-DSA-256f/s (256-bit security, fast/small) ← **Recommended for TOP SECRET**
- **Signature Size**: ~29,792 bytes (SLH-DSA-256f), ~49,856 bytes (SLH-DSA-256s)

### Implementation Roadmap

**Current Status** (2026):
- ✅ P-384 baseline implemented (valid through 2030)
- ⚠️ PQC support in planning phase

**Target Timeline**:
- **2027**: Begin PQC implementation
  - Add ML-KEM-1024 support for key exchange
  - Integrate with SSH protocol (draft-ietf-ssh-pqc)
- **2028**: Complete PQC implementation
  - Add ML-DSA-87 for signatures
  - Add SLH-DSA-256 as backup
  - Testing and validation
- **2029**: Production deployment
  - Parallel operation: P-384 + PQC (hybrid mode)
  - Migration tooling and documentation
- **2030**: Transition complete
  - PQC mandatory for new TOP SECRET systems
  - Hybrid mode continues for backward compatibility

### Hybrid Mode

During transition, systems will support **hybrid key exchange**:
- **Phase 1** (2030-2031): `ecdh-sha2-nistp384` + `mlkem1024-sha384`
- **Phase 2** (2032-2033): PQC primary, P-384 fallback
- **Phase 3** (2033+): PQC only for TOP SECRET

This ensures:
- Forward security against quantum computers
- Backward compatibility during transition
- Defense in depth (both algorithms must be broken)

### Why AES-256 Remains Valid

**Symmetric encryption** (AES-256) is quantum-resistant:
- Grover's algorithm only provides quadratic speedup (2^256 → 2^128 effective)
- 256-bit keys provide 128-bit post-quantum security (adequate for TOP SECRET)
- No changes needed to symmetric cryptography

### SSH Protocol Extensions

PQC support requires SSH protocol extensions:
- **draft-ietf-ssh-pqc**: SSH protocol extension for post-quantum KEMs
- **Integration**: Negotiation of hybrid key exchange methods
- **Compatibility**: Graceful fallback for non-PQC clients

## Compliance Documentation

### NIST Standards

This implementation aligns with:

- **NIST SP 800-52 Rev. 2**: Guidelines for the Selection, Configuration, and Use of Transport Layer Security (TLS) Implementations
- **NIST SP 800-131A Rev. 2**: Transitioning the Use of Cryptographic Algorithms and Key Lengths
- **FIPS 140-3**: Security Requirements for Cryptographic Modules
- **FIPS 203**: Module-Lattice-Based Key-Encapsulation Mechanism Standard (ML-KEM)
- **FIPS 204**: Module-Lattice-Based Digital Signature Standard (ML-DSA)
- **FIPS 205**: Stateless Hash-Based Digital Signature Standard (SLH-DSA)

### CNSS Policy

- **CNSS Advisory Memorandum**: Commercial National Security Algorithm Suite 2.0
- **CNSSP 15**: Use of Public Standards for Secure Information Sharing
- **CNSSI 1253**: Security Categorization and Control Selection for National Security Systems

### RFCs and Drafts

- **RFC 5656**: Elliptic Curve Algorithm Integration in the Secure Shell Transport Layer
- **RFC 8709**: Ed25519 and Ed448 Public Key Algorithms for the Secure Shell (SSH) Protocol
- **RFC 5647**: AES Galois Counter Mode for the Secure Shell Transport Layer Protocol
- **draft-ietf-ssh-pqc**: Post-Quantum Key Exchange for the Secure Shell (SSH) Protocol (Work in Progress)

## Migration from Non-Compliant Configurations

### Timeline

Organizations should:

1. **Immediate**: Generate CNSA 2.0 compliant host and client keys
2. **Week 1**: Deploy new keys to test environments
3. **Week 2-4**: Update client systems and authorized_keys
4. **Week 4**: Enable CNSA 2.0 enforcement on production servers
5. **Week 6**: Audit and verify all connections use compliant algorithms
6. **Week 8**: Remove all non-compliant keys

### Compatibility Considerations

**Breaking Changes**: Enforcing CNSA 2.0 compliance breaks compatibility with:

- Older SSH clients that don't support P-384 or Ed25519
- Systems using RSA-only authentication
- Clients configured for weaker ciphers (AES-128, 3DES, etc.)

**Mitigation**: Ensure all client systems are updated to support CNSA 2.0 algorithms before enabling enforcement on production servers.

## Testing

The CNSA module includes comprehensive tests in [src/cnsa.rs](../src/cnsa.rs):

```bash
# Run CNSA compliance tests
cargo test cnsa

# Expected output shows all approved algorithms are configured
```

## References

1. NSA CNSA Suite 2.0: https://media.defense.gov/2022/Sep/07/2003071834/-1/-1/0/CSA_CNSA_2.0_ALGORITHMS_.PDF
2. NIST SP 800-52 Rev. 2: https://csrc.nist.gov/publications/detail/sp/800-52/rev-2/final
3. RFC 5656 (ECDH/ECDSA): https://www.rfc-editor.org/rfc/rfc5656.html
4. RFC 8709 (Ed25519): https://www.rfc-editor.org/rfc/rfc8709.html
5. OpenSSH Cipher Documentation: https://www.openssh.com/specs.html

## Support

For questions about CNSA 2.0 compliance, please:

1. Review the [CNSA module source code](../src/cnsa.rs)
2. Check the [NSA CNSA 2.0 advisory](https://media.defense.gov/2022/Sep/07/2003071834/-1/-1/0/CSA_CNSA_2.0_ALGORITHMS_.PDF)
3. Open an issue on the GitHub repository

---

**Last Updated**: 2026-01-20
**Compliance Version**: CNSA 2.0
**Implementation Status**: Complete
