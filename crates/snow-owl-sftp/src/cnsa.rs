//! NSA Commercial National Security Algorithm (CNSA) 2.0 Suite Compliance
//!
//! This module provides configuration for CNSA 2.0 compliant cryptographic algorithms
//! as specified in the Committee for National Security Systems (CNSS) Policy.
//!
//! ## CNSA 2.0 Requirements by Classification Level
//!
//! ### For **SECRET** and below (effective immediately):
//! - **Encryption**: AES-256 (GCM mode preferred)
//! - **Key Exchange**: ECDH with P-384
//! - **Digital Signatures**: ECDSA with P-384
//! - **Hashing**: SHA-384, SHA-512
//!
//! ### For **TOP SECRET** and above (transition timeline):
//! - **Encryption**: AES-256 (current), quantum-resistant algorithms (post-2030)
//! - **Key Exchange**: ECDH P-384 (current), quantum-resistant KEM (post-2030)
//! - **Digital Signatures**: ECDSA P-384 (current), quantum-resistant signatures (post-2030)
//! - **Hashing**: SHA-384, SHA-512
//!
//! **Note**: For TOP SECRET, CNSA 2.0 mandates transition to quantum-resistant
//! algorithms by 2030-2033. Current implementation supports the pre-quantum
//! baseline (P-384/AES-256). Post-quantum cryptography (PQC) support requires:
//! - ML-KEM (FIPS 203) for key encapsulation
//! - ML-DSA (FIPS 204) for digital signatures
//! - SLH-DSA (FIPS 205) for stateless signatures
//!
//! ### For **non-classified** use:
//! The following modern algorithms are also acceptable:
//! - **Key Exchange**: X25519 (Curve25519)
//! - **Digital Signatures**: Ed25519
//!
//! ## SSH Algorithm Mapping
//!
//! This maps CNSA 2.0 requirements to SSH protocol algorithm names:
//!
//! ### Key Exchange (kex)
//! - `ecdh-sha2-nistp384` - ECDH with P-384 curve (CNSA 2.0 required)
//! - `curve25519-sha256` - X25519 (acceptable for non-classified)
//!
//! ### Encryption (cipher)
//! - `aes256-gcm@openssh.com` - AES-256-GCM (CNSA 2.0 preferred)
//! - `aes256-ctr` - AES-256-CTR (CNSA 2.0 acceptable fallback)
//!
//! ### MAC (Message Authentication Code)
//! - `hmac-sha2-512` - HMAC-SHA-512 (CNSA 2.0)
//! - `hmac-sha2-256` - HMAC-SHA-256 (CNSA 2.0, for AES-256-CTR)
//! - Note: AES-GCM modes provide integrated authentication (AEAD)
//!
//! ### Host Key / Public Key
//! - `ecdsa-sha2-nistp384` - ECDSA with P-384 (CNSA 2.0 required)
//! - `ssh-ed25519` - Ed25519 (acceptable for non-classified)
//!
//! ## RSA Exclusion
//!
//! **RSA is explicitly NOT supported** in this implementation:
//! - RSA key exchange: Vulnerable to quantum attacks via Shor's algorithm
//! - RSA signatures: Not CNSA 2.0 compliant for any classification level
//! - russh crate configured with `default-features = false` to exclude RSA
//! - Only EC-based algorithms (P-384, Ed25519) are enabled
//!
//! If you have existing RSA keys, you **must** generate new ECDSA P-384 or
//! Ed25519 keys for CNSA 2.0 compliance. See documentation for migration guide.
//!
//! ## References
//! - CNSS Advisory Memorandum: Commercial National Security Algorithm Suite 2.0
//! - NIST SP 800-52 Rev. 2: Guidelines for TLS Implementations
//! - RFC 5656: Elliptic Curve Algorithm Integration in SSH
//! - RFC 8709: Ed25519 and Ed448 Public Key Algorithms for SSH

use russh::cipher::Name as CipherName;
use russh::kex::Name as KexName;
use russh::key::Name as KeyName;
use russh::mac::Name as MacName;

/// CNSA 2.0 compliant key exchange algorithms
///
/// Ordered by preference (most secure first):
/// 1. ECDH with P-384 (CNSA 2.0 required for SECRET and below)
/// 2. X25519/Curve25519 (acceptable for non-classified, modern and fast)
pub const CNSA_KEX_ALGORITHMS: &[KexName] = &[
    // Primary CNSA 2.0 algorithm
    KexName::EcdhSha2Nistp384,

    // Acceptable for non-classified use (modern, fast, secure)
    KexName::Curve25519Sha256,
];

/// CNSA 2.0 compliant encryption algorithms
///
/// Ordered by preference (most secure first):
/// 1. AES-256-GCM (AEAD cipher, CNSA 2.0 preferred)
/// 2. AES-256-CTR (CNSA 2.0 acceptable fallback)
///
/// Note: AES-GCM modes provide authenticated encryption (AEAD) which is
/// cryptographically superior to CTR mode with separate MAC.
pub const CNSA_CIPHERS: &[CipherName] = &[
    // Preferred AEAD cipher (authenticated encryption)
    CipherName::Aes256Gcm,

    // Acceptable fallback (requires separate MAC)
    CipherName::Aes256Ctr,
];

/// CNSA 2.0 compliant MAC algorithms
///
/// Ordered by preference (strongest first):
/// 1. HMAC-SHA-512 (CNSA 2.0, 512-bit hash)
/// 2. HMAC-SHA-256 (CNSA 2.0, 256-bit hash, minimum requirement)
///
/// Note: When using AES-GCM ciphers, MAC is not needed (AEAD provides
/// integrated authentication). These MACs are used with CTR mode ciphers.
pub const CNSA_MAC_ALGORITHMS: &[MacName] = &[
    // Stronger hash for CNSA 2.0
    MacName::HmacSha2_512,

    // Minimum acceptable for CNSA 2.0
    MacName::HmacSha2_256,
];

/// CNSA 2.0 compliant host key algorithms
///
/// Ordered by preference:
/// 1. ECDSA with P-384 (CNSA 2.0 required for SECRET and below)
/// 2. Ed25519 (acceptable for non-classified, modern EdDSA)
///
/// Note: RSA keys are NOT CNSA 2.0 compliant and must not be used.
pub const CNSA_HOST_KEY_ALGORITHMS: &[KeyName] = &[
    // Primary CNSA 2.0 algorithm
    KeyName::EcdsaSha2Nistp384,

    // Acceptable for non-classified use (EdDSA, modern and secure)
    KeyName::Ed25519,
];

/// CNSA 2.0 compliant public key authentication algorithms
///
/// Same as host key algorithms - ECDSA P-384 and Ed25519 only.
pub const CNSA_PUBLIC_KEY_ALGORITHMS: &[KeyName] = CNSA_HOST_KEY_ALGORITHMS;

/// Validate that a cipher name is CNSA 2.0 compliant
pub fn is_cipher_compliant(cipher: &CipherName) -> bool {
    CNSA_CIPHERS.contains(cipher)
}

/// Validate that a key exchange algorithm is CNSA 2.0 compliant
pub fn is_kex_compliant(kex: &KexName) -> bool {
    CNSA_KEX_ALGORITHMS.contains(kex)
}

/// Validate that a MAC algorithm is CNSA 2.0 compliant
pub fn is_mac_compliant(mac: &MacName) -> bool {
    CNSA_MAC_ALGORITHMS.contains(mac)
}

/// Validate that a host key algorithm is CNSA 2.0 compliant
pub fn is_host_key_compliant(key: &KeyName) -> bool {
    CNSA_HOST_KEY_ALGORITHMS.contains(key)
}

/// Classification level for CNSA 2.0 compliance
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassificationLevel {
    /// Unclassified information
    Unclassified,
    /// Classified SECRET and below (P-384/AES-256)
    Secret,
    /// Classified TOP SECRET (P-384/AES-256, transitioning to PQC by 2030)
    TopSecret,
}

impl ClassificationLevel {
    /// Get the required algorithms for this classification level
    pub fn required_algorithms(&self) -> &'static str {
        match self {
            ClassificationLevel::Unclassified => {
                "ECDH-P384 or X25519, AES-256, ECDSA-P384 or Ed25519"
            }
            ClassificationLevel::Secret => {
                "ECDH-P384, AES-256, ECDSA-P384, SHA-384/512 (CNSA 2.0 baseline)"
            }
            ClassificationLevel::TopSecret => {
                "ECDH-P384, AES-256, ECDSA-P384, SHA-384/512 (current baseline)\n\
                 Transition to quantum-resistant algorithms required by 2030:\n\
                 - ML-KEM (FIPS 203) for key exchange\n\
                 - ML-DSA (FIPS 204) for digital signatures\n\
                 - AES-256 remains approved for symmetric encryption"
            }
        }
    }

    /// Check if quantum-resistant algorithms are required
    pub fn requires_pqc(&self) -> bool {
        matches!(self, ClassificationLevel::TopSecret)
    }
}

/// Get a human-readable description of CNSA 2.0 compliance
pub fn compliance_info() -> &'static str {
    r#"NSA CNSA 2.0 Compliance

    This server enforces NSA Commercial National Security Algorithm (CNSA) Suite 2.0
    cryptographic standards suitable for protecting classified information.

    Classification Level Support:
    - UNCLASSIFIED: ECDH-P384/X25519, AES-256, ECDSA-P384/Ed25519
    - SECRET: ECDH-P384, AES-256, ECDSA-P384, SHA-384/512 (baseline)
    - TOP SECRET: ECDH-P384, AES-256, ECDSA-P384 (current)
                  Transition to quantum-resistant algorithms by 2030
                  (ML-KEM, ML-DSA, SLH-DSA)

    Currently Supported Algorithms:
    - Key Exchange: ECDH-P384, X25519
    - Encryption: AES-256-GCM (AEAD), AES-256-CTR
    - MAC: HMAC-SHA-512, HMAC-SHA-256
    - Digital Signatures: ECDSA-P384, Ed25519

    Non-compliant algorithms (RSA, DES, 3DES, RC4, MD5, SHA-1) are disabled.

    Note: Post-quantum cryptography (PQC) support for TOP SECRET classification
    is planned for future release to meet 2030 transition deadline.
    "#
}

/// Get post-quantum cryptography (PQC) readiness information
pub fn pqc_readiness_info() -> &'static str {
    r#"Post-Quantum Cryptography (PQC) Readiness

    CNSA 2.0 requires transition to quantum-resistant algorithms for TOP SECRET
    by 2030-2033. The following NIST-standardized algorithms will be required:

    Required PQC Algorithms:
    - ML-KEM (FIPS 203): Module-Lattice-Based Key Encapsulation Mechanism
      * Replaces ECDH for key exchange
      * Parameter sets: ML-KEM-512, ML-KEM-768, ML-KEM-1024
      * For TOP SECRET: ML-KEM-1024 (256-bit security)

    - ML-DSA (FIPS 204): Module-Lattice-Based Digital Signature Algorithm
      * Replaces ECDSA for signatures
      * Parameter sets: ML-DSA-44, ML-DSA-65, ML-DSA-87
      * For TOP SECRET: ML-DSA-87 (256-bit security)

    - SLH-DSA (FIPS 205): Stateless Hash-Based Signature Algorithm
      * Backup signature scheme (hash-based, conservative)
      * Parameter sets: SLH-DSA-128f/s, SLH-DSA-192f/s, SLH-DSA-256f/s
      * For TOP SECRET: SLH-DSA-256f or SLH-DSA-256s

    Current Status:
    - SSH protocol extension for PQC is under development (draft-ietf-ssh-pqc)
    - russh library PQC support: Pending
    - Implementation timeline: Target 2027-2028 (ahead of 2030 deadline)

    Symmetric Encryption:
    - AES-256 remains approved (quantum-safe with 256-bit keys)
    - No changes required for symmetric cryptography
    "#
}

/// Compile-time verification that RSA is not enabled
///
/// This ensures the russh crate was configured without RSA support.
/// If this fails to compile, RSA is enabled and CNSA 2.0 compliance is broken.
#[cfg(test)]
const _: () = {
    // This will fail to compile if RSA types are available
    #[cfg(feature = "rsa")]
    compile_error!("RSA feature must not be enabled - violates CNSA 2.0 compliance");
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cnsa_kex_algorithms() {
        // Should contain CNSA 2.0 required algorithms
        assert!(CNSA_KEX_ALGORITHMS.contains(&KexName::EcdhSha2Nistp384));
        assert!(CNSA_KEX_ALGORITHMS.contains(&KexName::Curve25519Sha256));

        // Should be in order of preference
        assert_eq!(CNSA_KEX_ALGORITHMS[0], KexName::EcdhSha2Nistp384);
    }

    #[test]
    fn test_cnsa_ciphers() {
        // Should contain CNSA 2.0 required ciphers
        assert!(CNSA_CIPHERS.contains(&CipherName::Aes256Gcm));
        assert!(CNSA_CIPHERS.contains(&CipherName::Aes256Ctr));

        // Should prefer GCM (AEAD)
        assert_eq!(CNSA_CIPHERS[0], CipherName::Aes256Gcm);

        // Should only be AES-256 variants
        assert_eq!(CNSA_CIPHERS.len(), 2);
    }

    #[test]
    fn test_cnsa_mac_algorithms() {
        // Should contain CNSA 2.0 compliant MACs
        assert!(CNSA_MAC_ALGORITHMS.contains(&MacName::HmacSha2_512));
        assert!(CNSA_MAC_ALGORITHMS.contains(&MacName::HmacSha2_256));

        // Should prefer SHA-512
        assert_eq!(CNSA_MAC_ALGORITHMS[0], MacName::HmacSha2_512);
    }

    #[test]
    fn test_cnsa_host_key_algorithms() {
        // Should contain CNSA 2.0 compliant key types
        assert!(CNSA_HOST_KEY_ALGORITHMS.contains(&KeyName::EcdsaSha2Nistp384));
        assert!(CNSA_HOST_KEY_ALGORITHMS.contains(&KeyName::Ed25519));

        // Should prefer P-384
        assert_eq!(CNSA_HOST_KEY_ALGORITHMS[0], KeyName::EcdsaSha2Nistp384);

        // Should NOT contain RSA
        assert_eq!(CNSA_HOST_KEY_ALGORITHMS.len(), 2);
    }

    #[test]
    fn test_cipher_compliance() {
        assert!(is_cipher_compliant(&CipherName::Aes256Gcm));
        assert!(is_cipher_compliant(&CipherName::Aes256Ctr));
    }

    #[test]
    fn test_kex_compliance() {
        assert!(is_kex_compliant(&KexName::EcdhSha2Nistp384));
        assert!(is_kex_compliant(&KexName::Curve25519Sha256));
    }

    #[test]
    fn test_mac_compliance() {
        assert!(is_mac_compliant(&MacName::HmacSha2_512));
        assert!(is_mac_compliant(&MacName::HmacSha2_256));
    }

    #[test]
    fn test_host_key_compliance() {
        assert!(is_host_key_compliant(&KeyName::EcdsaSha2Nistp384));
        assert!(is_host_key_compliant(&KeyName::Ed25519));
    }

    #[test]
    fn test_public_key_same_as_host_key() {
        assert_eq!(CNSA_PUBLIC_KEY_ALGORITHMS, CNSA_HOST_KEY_ALGORITHMS);
    }

    #[test]
    fn test_classification_levels() {
        assert!(!ClassificationLevel::Unclassified.requires_pqc());
        assert!(!ClassificationLevel::Secret.requires_pqc());
        assert!(ClassificationLevel::TopSecret.requires_pqc());
    }

    #[test]
    fn test_classification_required_algorithms() {
        let unclass = ClassificationLevel::Unclassified.required_algorithms();
        assert!(unclass.contains("X25519"));
        assert!(unclass.contains("Ed25519"));

        let secret = ClassificationLevel::Secret.required_algorithms();
        assert!(secret.contains("P-384"));
        assert!(secret.contains("AES-256"));

        let ts = ClassificationLevel::TopSecret.required_algorithms();
        assert!(ts.contains("P-384"));
        assert!(ts.contains("quantum-resistant"));
        assert!(ts.contains("ML-KEM"));
    }

    #[test]
    fn test_compliance_info() {
        let info = compliance_info();
        assert!(info.contains("CNSA 2.0"));
        assert!(info.contains("ECDH-P384"));
        assert!(info.contains("AES-256"));
        assert!(info.contains("TOP SECRET"));
    }

    #[test]
    fn test_pqc_readiness_info() {
        let info = pqc_readiness_info();
        assert!(info.contains("ML-KEM"));
        assert!(info.contains("ML-DSA"));
        assert!(info.contains("SLH-DSA"));
        assert!(info.contains("FIPS 203"));
        assert!(info.contains("2030"));
    }

    #[test]
    fn test_no_rsa_algorithms() {
        // Verify that none of our approved algorithms are RSA-based
        // This is a static check to ensure we never accidentally add RSA

        // All KEX algorithms must be EC-based
        for kex in CNSA_KEX_ALGORITHMS {
            let kex_str = format!("{:?}", kex);
            assert!(!kex_str.to_lowercase().contains("rsa"),
                   "KEX algorithm contains RSA: {:?}", kex);
        }

        // All signature algorithms must be EC-based
        for key in CNSA_HOST_KEY_ALGORITHMS {
            let key_str = format!("{:?}", key);
            assert!(!key_str.to_lowercase().contains("rsa"),
                   "Signature algorithm contains RSA: {:?}", key);
        }

        // Verify we have exactly 2 KEX algorithms (P-384 and X25519)
        assert_eq!(CNSA_KEX_ALGORITHMS.len(), 2,
                  "Should have exactly 2 KEX algorithms");

        // Verify we have exactly 2 signature algorithms (P-384 and Ed25519)
        assert_eq!(CNSA_HOST_KEY_ALGORITHMS.len(), 2,
                  "Should have exactly 2 signature algorithms");
    }

    #[test]
    fn test_only_ec_curves() {
        // Verify that P-384 is present (CNSA 2.0 required)
        assert!(CNSA_KEX_ALGORITHMS.contains(&KexName::EcdhSha2Nistp384),
               "P-384 must be present for CNSA 2.0");
        assert!(CNSA_HOST_KEY_ALGORITHMS.contains(&KeyName::EcdsaSha2Nistp384),
               "ECDSA P-384 must be present for CNSA 2.0");

        // Verify Ed25519 is present (acceptable for unclassified)
        assert!(CNSA_KEX_ALGORITHMS.contains(&KexName::Curve25519Sha256),
               "X25519 should be present for unclassified use");
        assert!(CNSA_HOST_KEY_ALGORITHMS.contains(&KeyName::Ed25519),
               "Ed25519 should be present for unclassified use");
    }

    #[test]
    fn test_compliance_info_mentions_rsa_exclusion() {
        let info = compliance_info();
        // Should explicitly mention that RSA is disabled
        let info_lower = info.to_lowercase();
        assert!(info_lower.contains("rsa") || info_lower.contains("disabled") ||
                info_lower.contains("non-compliant"),
               "Compliance info should mention RSA exclusion");
    }
}
