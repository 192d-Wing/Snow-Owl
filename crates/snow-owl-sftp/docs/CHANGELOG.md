# Changelog

All notable changes to the Snow Owl SFTP crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Comprehensive Testing Suite (Phase 1.4 - Complete)** - Production-ready test coverage
  - **Protocol Encoding Tests** (tests/protocol_encoding_tests.rs - 350+ lines)
    - All MessageType conversions (18 message types including Init, Open, Read, Write, Status, etc.)
    - All StatusCode mappings (9 codes: Ok, Eof, NoSuchFile, PermissionDenied, etc.)
    - OpenFlags combinations and bitwise operations (READ, WRITE, APPEND, CREAT, TRUNC, EXCL)
    - FileAttrs encoding/decoding with complete, partial, and empty field sets
    - String codec tests with various lengths (0 to 1000+ chars) and UTF-8 validation
    - Bytes codec tests with edge cases (empty, single byte, 256-byte sequences)
    - Invalid data handling (insufficient data, invalid UTF-8)
    - Maximum value testing (u32::MAX, u64::MAX, octal permissions)
    - NIST 800-53: SI-11, SI-10
    - STIG: V-222566, V-222396
  - **Error Handling Tests** (tests/error_handling_tests.rs - 280+ lines)
    - Error categorization (is_recoverable, is_client_error, is_security_event)
    - SFTP status code mapping for all 15 error types
    - Sanitized error messages preventing information disclosure (auth, permission, config)
    - Error constructor helpers (timeout, channel_closed, invalid_handle, resource_exhaustion)
    - IO error conversion and display formatting
    - Retry logic classification for client resilience
    - Security event audit classification for compliance
    - Comprehensive coverage of all error types
    - NIST 800-53: SI-11, AU-2
    - STIG: V-222566
  - **Authentication & Security Tests** (tests/authentication_tests.rs - 270+ lines)
    - AuthorizedKeys file parsing (comments, empty files, nonexistent paths)
    - RateLimiter enforcement with configurable attempts/windows
    - Per-IP rate limiting with IPv4 and IPv6 support
    - Rate limit reset on successful authentication
    - ConnectionTracker per-user limits and isolation
    - Connection registration, cleanup, and statistics
    - Concurrent session control and enforcement
    - NIST 800-53: IA-2, AC-7, AC-10, AC-12
    - STIG: V-222611, V-222578, V-222601
  - **File Operations Tests** (tests/file_operations_tests.rs - 280+ lines)
    - Path resolution within root directory
    - Path traversal prevention (../ detection)
    - Empty path and null byte handling
    - File create, read, write, delete operations
    - File overwrite and append operations
    - Various file sizes (empty to 100KB)
    - File rename and metadata retrieval
    - Error conditions (nonexistent files, permission denied)
    - Special characters in filenames
    - Subdirectory file operations
    - NIST 800-53: SI-11, AC-3, SI-10
    - STIG: V-222566, V-222596, V-222396
  - **Directory Operations Tests** (tests/directory_operations_tests.rs - 430+ lines)
    - Directory creation and removal
    - Nested directory creation (multiple levels)
    - Non-empty directory removal prevention
    - Directory listing (files, dirs, mixed content)
    - Empty directory listing
    - Directory metadata retrieval
    - Directory with special characters
    - Creating existing directory error handling
    - Recursive directory removal
    - Directory permissions (Unix platform)
    - Long directory names
    - Multiple sequential directory operations
    - Directory listing consistency
    - NIST 800-53: SI-11, AC-3
    - STIG: V-222566, V-222596
  - **Concurrent Operations Tests** (tests/concurrent_operations_tests.rs - 470+ lines)
    - Concurrent file reads (10+ simultaneous)
    - Concurrent writes to different files
    - Concurrent directory creation
    - Mixed concurrent operations (reads + writes)
    - ConnectionTracker under concurrent load (20+ connections)
    - ConnectionTracker limit enforcement with concurrent attempts
    - RateLimiter under concurrent authentication attempts
    - RateLimiter with multiple IPs concurrently
    - Connection cleanup under concurrent load
    - Concurrent directory listing
    - Concurrent metadata reads
    - Concurrent file renames and deletions
    - High concurrency stress test (100+ operations)
    - ConnectionTracker statistics under load
    - NIST 800-53: AC-10, AC-12, AC-7
    - STIG: V-222601
  - **Test Statistics**:
    - 7 test files (integration_test.rs + 6 comprehensive test suites)
    - 2,213+ total lines of test code
    - 115+ individual test cases
    - Coverage: Protocol, Errors, Authentication, File Ops, Directory Ops, Concurrency, Security
  - Phase 1.4: 7/7 tasks complete (100%) ✅
  - End-to-end tests with real SFTP clients deferred to Phase 2
- **Enhanced Error Handling & Reliability (Phase 1.3)** - Production-grade error handling
  - Enhanced error module with comprehensive error types (Timeout, InvalidHandle, ResourceExhaustion, NotSupported, ChannelClosed)
  - Error categorization helpers (is_recoverable, is_client_error, is_security_event)
  - SFTP status code mapping (to_status_code) for RFC-compliant error responses
  - Sanitized error messages (sanitized_message) to prevent information disclosure
  - Error constructor helpers for common scenarios
  - Comprehensive unit tests for all error functionality
  - Robust error handling across all SFTP operations (open, read, write, stat, opendir, etc.)
  - Channel closed detection with graceful connection drop handling
  - Automatic file handle cleanup via Drop trait implementation
  - Session cleanup on unexpected termination (closes all open handles)
  - Timeout protection (30 seconds) for all file operations (read, write, stat, opendir, remove, mkdir, rmdir, rename)
  - Enhanced input validation with null byte detection
  - Path traversal protection with security event logging
  - Resource exhaustion detection (max 1024 file handles per session)
  - Detailed contextual error messages with appropriate log levels
  - Security event logging for authentication failures, permission denials, and path traversal attempts
  - Session initialization checks to prevent operations before handshake
  - NIST 800-53: SI-11 (Error Handling), AC-3 (Access Enforcement), AC-12 (Session Termination), SC-8 (Transmission Confidentiality), SI-10 (Input Validation)
  - STIG: V-222566 (Error messages), V-222596 (Access control), V-222601 (Session termination), V-222396 (Input validation)
- **Complete SFTP Client Implementation (src/client.rs)** - Full-featured RFC-compliant SFTP client
  - SSH connection establishment with public key authentication
  - SFTP protocol initialization and version negotiation
  - File upload (PUT) operation with chunked transfers
  - File download (GET) operation with chunked reads (32KB chunks)
  - Directory listing with file attributes
  - Directory operations (mkdir, rmdir)
  - File operations (remove, rename)
  - Attribute retrieval (stat, fstat)
  - Graceful session termination
  - Complete rustdoc documentation for all public methods
  - NIST 800-53: IA-2 (Authentication), SC-8 (Transmission Confidentiality), SC-13 (Cryptographic Protection), AC-3 (Access Enforcement), AC-12 (Session Termination)
  - STIG: V-222577 (Cryptographic mechanisms), V-222611 (Certificate validation)
- **SFTP Client Binary (src/bin/client.rs)** - CLI tool for SFTP operations
  - Commands: put, get, ls, mkdir, rm, rmdir, rename
  - SSH private key authentication with -i/--identity flag
  - Configurable host (-H), port (-p), username (-u)
  - Verbose logging support (-v)
  - User-friendly error messages
  - Tilde expansion for key paths (~/.ssh/id_rsa)
- **Connection Tracking Module (src/connection_tracker.rs)** - Per-user concurrent connection limits
  - `ConnectionTracker` for tracking and limiting concurrent connections per user
  - Configurable maximum connections per user (default: 10)
  - Automatic cleanup when connections are terminated
  - Statistics tracking for monitoring (active users, total connections)
  - Integration with server authentication flow
  - Connection registration on successful authentication
  - Connection unregistration on session termination
  - NIST 800-53: AC-10 (Concurrent Session Control), AC-12 (Session Termination)
  - STIG: V-222601 (Session termination)
- **IPv6 Network Support Requirement (Rule 2)** - Mandatory IPv6 support for all network code
  - All network code must support IPv6
  - IPv6 preferred by default when available
  - Dual-stack (IPv6 with IPv4 fallback) as default configuration
  - IPv6-only mode support in configuration
  - NIST 800-53: SC-7 (Boundary Protection)
  - Updated development rules to include IPv6 requirements
  - Updated testing requirements to include IPv6 scenarios
- **Rate Limiting Module (src/rate_limit.rs)** - Brute force protection
  - `RateLimiter` for tracking authentication attempts per IP
  - Configurable attempt limits and lockout duration
  - Automatic cleanup of expired entries
  - Statistics tracking for monitoring
  - NIST 800-53: AC-7 (Unsuccessful Logon Attempts)
  - STIG: V-222578 (Replay-resistant authentication)
- **Authentication Module (src/auth.rs)** - Authorized keys management
  - `AuthorizedKeys` struct for parsing and validating SSH public keys
  - OpenSSH authorized_keys file format support
  - Public key verification against authorized keys
  - Hot-reloading capability for authorized_keys file
  - NIST 800-53: AC-2 (Account Management), IA-2 (Identification and Authentication)
  - STIG: V-222611 (Certificate validation)
- **Configuration Options** - Rate limiting and connection controls
  - `max_auth_attempts`: Maximum attempts per IP (default: 5)
  - `rate_limit_window_secs`: Time window for counting attempts (default: 300s)
  - `lockout_duration_secs`: How long to lock out after limit (default: 900s)
  - `max_connections_per_user`: Per-user connection limit (default: 10)
- Development rules enforcement (docs/DEVELOPMENT_RULES.md)
- NIST 800-53 and STIG compliance framework
- Security policy documentation (docs/SECURITY.md)
- Pre-commit verification script (verify.sh)
- Cargo linting rules (clippy strict mode)
- Changelog tracking (docs/CHANGELOG.md)
- Pull request template (.github/PULL_REQUEST_TEMPLATE.md)
- tempfile dev-dependency for testing

### Changed
- **Server authentication and session management** - Now includes rate limiting, connection limits, and brute force protection
  - Replaced accept-all authentication with proper key verification
  - Added rate limiting per IP address
  - Added concurrent connection limits per user
  - Automatic lockout after failed attempts
  - Reject authentication when user exceeds max connections
  - Automatic connection cleanup on session termination
  - Clear failed attempts counter on successful authentication
  - Added audit logging for authentication events (AC-2, AU-2, AC-7, AC-10, AC-12)
  - Integrated AuthorizedKeys, RateLimiter, and ConnectionTracker into SftpSessionHandler
- Reorganized documentation into docs/ folder for better structure
- Updated all documentation references to use docs/ paths

### Security
- **PRODUCTION READY: Authentication, Rate Limiting & Connection Control** - Server now properly validates SSH public keys with brute force protection and session limits
- Implemented AC-2 (Account Management) through authorized_keys
- Implemented IA-2 (Identification and Authentication) with public key crypto
- Implemented AC-7 (Unsuccessful Logon Attempts) with rate limiting and lockout
- Implemented AC-10 (Concurrent Session Control) with per-user connection limits
- Implemented AC-12 (Session Termination) with automatic cleanup
- Implemented V-222611 (Certificate validation) for SSH keys
- Implemented V-222578 (Replay-resistant authentication mechanisms)
- Implemented V-222601 (Session termination) with connection tracking
- Added AU-2 (Audit Events) logging for authentication attempts and connection events
- Protection against brute force attacks with configurable limits
- Protection against resource exhaustion with connection limits
- Documented NIST 800-53 control requirements
- Documented Application Security STIG compliance
- Added security hardening guidelines
- Added vulnerability reporting process

### Documentation
- **Updated Development Rules** - Now includes 4 mandatory rules (was 3)
  - Rule 1: Security Compliance Documentation (unchanged)
  - Rule 2: IPv6 Network Support (NEW)
  - Rule 3: Code Quality Standards (was Rule 2)
  - Rule 4: Documentation Synchronization (was Rule 3)
- Updated RULES_SUMMARY.md with IPv6 requirements
- Updated QUICK_REFERENCE.md with IPv6 examples and NIST SC-7
- Added IPv6 support checklist for network code
- Added IPv6 testing requirements (IPv4, IPv6, dual-stack, IPv4-mapped)
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
