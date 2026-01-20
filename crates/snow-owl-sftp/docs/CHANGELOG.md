# Changelog

All notable changes to the Snow Owl SFTP crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- Reorganized documentation into docs/ folder for better structure
- Updated all documentation references to use docs/ paths

### Added
- Development rules enforcement (docs/DEVELOPMENT_RULES.md)
- NIST 800-53 and STIG compliance framework
- Security policy documentation (docs/SECURITY.md)
- Pre-commit verification script (verify.sh)
- Cargo linting rules (clippy strict mode)
- Changelog tracking (docs/CHANGELOG.md)
- Pull request template (.github/PULL_REQUEST_TEMPLATE.md)

### Security
- Documented NIST 800-53 control requirements
- Documented Application Security STIG compliance
- Added security hardening guidelines
- Added vulnerability reporting process

### Documentation
- Created comprehensive development rules (6000+ words)
- Added security compliance documentation
- Enhanced README with security warnings
- Added quick reference card for developers
- Added rules summary for easy onboarding
- Organized all documentation in docs/ folder

## [0.1.0] - 2026-01-19

### Added
- Initial SFTP protocol implementation (version 3)
- RFC 4251-4254 compliant SSH/SFTP support
- Server implementation with core operations:
  - File operations: OPEN, READ, WRITE, CLOSE, REMOVE, RENAME
  - Directory operations: OPENDIR, READDIR, MKDIR, RMDIR
  - File attributes: STAT, LSTAT, FSTAT, REALPATH
- Client implementation structure (placeholder)
- Configuration system with TOML support
- Server and client binaries
- Path traversal protection (AC-3, V-222596)
- SSH authentication via russh library (IA-2)
- Session timeout support (AC-11, AC-12)
- Input validation (SI-10, V-222396)
- Secure error handling (SI-11, V-222566)

### Security
- Path traversal prevention in file operations
- SSH encryption for all data transmission (SC-8)
- Public key authentication support (IA-2)
- Configurable session timeouts (AC-12)
- Input validation for all SFTP messages (SI-10)

### Documentation
- Comprehensive README with features and usage
- RFC compliance documentation (RFC_COMPLIANCE.md)
- Development roadmap (ROADMAP.md)
- Quick start guide (QUICKSTART.md)
- Example configuration file
- Integration tests

### Dependencies
- russh 0.56: SSH protocol implementation
- russh-keys 0.49: SSH key handling
- tokio: Async runtime
- bytes: Zero-copy buffers
- thiserror: Error handling
- tracing: Structured logging

### Known Limitations
- ⚠️ Accepts all public key authentication (development mode)
- ⚠️ No authorized_keys verification
- ⚠️ No rate limiting
- ⚠️ Basic audit logging only
- ⚠️ **NOT PRODUCTION READY** - See SECURITY.md

### NIST 800-53 Controls Implemented
- AC-2: Account Management (partial)
- AC-3: Access Enforcement (path traversal protection)
- AC-11: Session Lock (timeouts)
- AC-12: Session Termination (automatic cleanup)
- AU-2: Audit Events (basic logging)
- AU-3: Content of Audit Records (structured logs)
- IA-2: Identification and Authentication (SSH keys)
- SC-8: Transmission Confidentiality (SSH encryption)
- SC-13: Cryptographic Protection (via russh)
- SI-10: Information Input Validation (all inputs)
- SI-11: Error Handling (secure error messages)

### STIG Findings Addressed
- V-222396: Input validation implemented
- V-222566: Secure error message handling
- V-222577: Cryptographic protection via SSH
- V-222596: Authorization enforcement (path checks)
- V-222601: Session termination support
- V-222602: Session ID protection via SSH

## Release Notes

### Version 0.1.0 - Initial Release

This is the initial release of Snow Owl SFTP, providing a foundational implementation of the SFTP protocol. **This version is for development and testing only.**

**What Works:**
- Complete SFTP v3 protocol implementation
- File upload, download, delete, rename
- Directory creation, listing, removal
- RFC-compliant message encoding/decoding
- SSH encryption and authentication
- Configuration management

**What's Missing:**
- Production-ready authentication
- Rate limiting and abuse prevention
- Advanced audit logging
- File attribute modification (SETSTAT)
- Symbolic link support
- Performance optimizations

**Next Steps:**
See ROADMAP.md Phase 1 for planned improvements:
- Authorized keys verification (Phase 1.1)
- Complete client implementation (Phase 1.2)
- Enhanced error handling (Phase 1.3)
- Comprehensive testing (Phase 1.4)

**Security Notice:**
This version should NOT be deployed in production environments. See SECURITY.md for current limitations and mitigation strategies.

---

## Version History

| Version | Date | Status | Notes |
|---------|------|--------|-------|
| 0.1.0 | 2026-01-19 | Released | Initial implementation |
| 0.2.0 | TBD | Planned | Authentication + Client |
| 1.0.0 | TBD | Planned | Production ready |

---

## Migration Guides

### Upgrading from 0.1.0 to 0.2.0 (When Released)

**Breaking Changes:**
- TBD

**New Features:**
- TBD

**Deprecations:**
- TBD

---

## Contributing

When adding changelog entries:

1. **Format**: Follow [Keep a Changelog](https://keepachangelog.com/)
2. **Categories**: Added, Changed, Deprecated, Removed, Fixed, Security
3. **Detail**: Include file references and NIST/STIG controls
4. **Links**: Link to relevant issues/PRs
5. **Timing**: Update with every commit

### Example Entry

```markdown
### Added
- Authorized keys verification in server.rs (V-222596, AC-2)
  Implements proper public key validation against authorized_keys file.
  Closes #123

### Security
- Fix path traversal in resolve_path() (CVE-XXXX-XXXXX)
  Added additional canonicalization check. Reported by @security-researcher
```

---

**Maintained by**: Snow Owl Contributors
**Last Updated**: 2026-01-19
