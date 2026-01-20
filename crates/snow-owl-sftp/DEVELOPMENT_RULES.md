# Snow Owl SFTP Development Rules

**MANDATORY REQUIREMENTS FOR ALL CODE CHANGES**

These rules MUST be followed for every action, commit, and pull request in this crate.

---

## 🔒 Rule 1: Security Compliance Documentation

**ALL code MUST include NIST Controls and Application Development STIG comments**

### Requirements

Every security-relevant code section must be documented with:
1. **NIST Control References** - Applicable NIST 800-53 controls
2. **Application Development STIG** - Relevant STIG findings/requirements
3. **Implementation Notes** - How the control is satisfied

### Template Format

```rust
// NIST 800-53: AC-3 (Access Enforcement)
// STIG: V-222596 - The application must enforce approved authorizations
// Implementation: Path traversal protection ensures users cannot access files
// outside their authorized root directory
fn resolve_path(&self, path: &str) -> Result<PathBuf> {
    // ... implementation
}
```

### Required Controls Coverage

#### Authentication (AC - Access Control)
- **AC-2**: Account Management
- **AC-3**: Access Enforcement
- **AC-7**: Unsuccessful Logon Attempts
- **AC-11**: Session Lock
- **AC-12**: Session Termination

#### Audit and Accountability (AU)
- **AU-2**: Audit Events
- **AU-3**: Content of Audit Records
- **AU-8**: Time Stamps
- **AU-9**: Protection of Audit Information
- **AU-12**: Audit Generation

#### Identification and Authentication (IA)
- **IA-2**: Identification and Authentication
- **IA-3**: Device Identification and Authentication
- **IA-5**: Authenticator Management

#### System and Communications Protection (SC)
- **SC-8**: Transmission Confidentiality and Integrity
- **SC-13**: Cryptographic Protection
- **SC-23**: Session Authenticity

#### System and Information Integrity (SI)
- **SI-10**: Information Input Validation
- **SI-11**: Error Handling

### Application Development STIG Requirements

Must address these STIG findings:

1. **V-222396**: Applications must validate input
2. **V-222397**: Applications must not be vulnerable to SQL injection
3. **V-222401**: Applications must not be vulnerable to XML injection
4. **V-222402**: Applications must not be vulnerable to LDAP injection
5. **V-222566**: Applications must generate error messages with security implications
6. **V-222575**: Applications must protect audit information
7. **V-222576**: Applications must protect audit tools
8. **V-222577**: Applications must use cryptographic mechanisms
9. **V-222578**: Applications must implement replay-resistant authentication
10. **V-222596**: Applications must enforce approved authorizations
11. **V-222597**: Applications must enforce separation of duties
12. **V-222601**: Applications must terminate sessions
13. **V-222602**: Applications must protect session IDs
14. **V-222611**: Applications must validate certificates

### Checklist for Each File

Before committing ANY file:

- [ ] All authentication code has AC controls documented
- [ ] All authorization checks have AC-3 documented
- [ ] All input validation has SI-10 documented
- [ ] All error handling has SI-11 documented
- [ ] All audit/logging has AU controls documented
- [ ] All crypto operations have SC-13 documented
- [ ] All session management has AC-11, AC-12 documented
- [ ] Relevant STIG findings are referenced

---

## 🧹 Rule 2: Code Quality Standards

**ALL code MUST pass `cargo fmt` and `cargo clippy` with ZERO issues**

### Pre-Commit Requirements

Run these commands before EVERY commit:

```bash
# 1. Format code (MUST be run)
cargo fmt --package snow-owl-sftp

# 2. Check formatting (MUST pass)
cargo fmt --package snow-owl-sftp -- --check

# 3. Run clippy with strict settings (MUST have no warnings)
cargo clippy --package snow-owl-sftp -- -D warnings

# 4. Run clippy with pedantic (SHOULD pass, exceptions documented)
cargo clippy --package snow-owl-sftp -- -W clippy::pedantic
```

### Clippy Configuration

Add to `Cargo.toml`:

```toml
[lints.clippy]
# Deny all warnings
all = "deny"

# Pedantic lints we enforce
pedantic = "warn"
nursery = "warn"

# Specific denies
missing_docs = "deny"
missing_errors_doc = "deny"
missing_panics_doc = "deny"
missing_safety_doc = "deny"
undocumented_unsafe_blocks = "deny"

# Security-critical denies
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
unreachable = "deny"
```

### Formatting Standards

- **Line Length**: 100 characters maximum
- **Indentation**: 4 spaces (enforced by rustfmt)
- **Imports**: Grouped and sorted
- **Comments**: Full sentences with proper punctuation
- **Documentation**: Complete rustdoc for all public items

### Allowed Exceptions

Only these clippy exceptions are allowed (with justification):

```rust
// Allowed: Module has intentional complexity for protocol implementation
#[allow(clippy::too_many_lines)]
mod protocol {
    // ... complex protocol code
}

// Must document why each allow is needed
#[allow(clippy::module_name_repetitions)] // SFTP protocol requires repetition
```

### Quality Gates

Code MUST pass all these gates:

```bash
# Gate 1: Compilation
cargo build --package snow-owl-sftp

# Gate 2: Tests
cargo test --package snow-owl-sftp

# Gate 3: Format check
cargo fmt --package snow-owl-sftp -- --check

# Gate 4: Clippy (no warnings)
cargo clippy --package snow-owl-sftp -- -D warnings

# Gate 5: Documentation
cargo doc --package snow-owl-sftp --no-deps

# Gate 6: Security audit (run weekly)
cargo audit
```

---

## 📚 Rule 3: Documentation Synchronization

**ALL documentation MUST be updated at the end of EVERY action**

### Documentation Update Checklist

After ANY code change, update these files as applicable:

#### Always Update
- [ ] **Code Comments**: NIST/STIG comments added/updated
- [ ] **Rustdoc**: All public items have complete documentation
- [ ] **CHANGELOG.md**: Entry added for the change

#### Update When Applicable

**Features Added:**
- [ ] **README.md**: Feature list updated
- [ ] **ROADMAP.md**: Mark task as completed, update status
- [ ] **QUICKSTART.md**: Add usage example if user-facing

**Protocol Changes:**
- [ ] **RFC_COMPLIANCE.md**: Update compliance status
- [ ] **protocol.rs**: Update module documentation

**Configuration Changes:**
- [ ] **config.rs**: Update field documentation
- [ ] **config.example.toml**: Add example configuration
- [ ] **QUICKSTART.md**: Update configuration section

**Security Changes:**
- [ ] **SECURITY.md**: Document security implications (create if needed)
- [ ] **RFC_COMPLIANCE.md**: Update security section
- [ ] **README.md**: Update security features

**API Changes:**
- [ ] **lib.rs**: Update crate-level documentation
- [ ] **README.md**: Update API examples
- [ ] **MIGRATION.md**: Document breaking changes (create if needed)

**Performance Changes:**
- [ ] **ROADMAP.md**: Update performance section
- [ ] **README.md**: Update performance claims
- [ ] Add benchmarks if claiming improvement

**Bug Fixes:**
- [ ] **CHANGELOG.md**: Document the fix
- [ ] **Tests**: Add regression test
- [ ] **RFC_COMPLIANCE.md**: Update if compliance affected

### Documentation Standards

#### Rustdoc Requirements

```rust
/// Brief one-line description (ends with period).
///
/// Detailed explanation of what this does, how it works,
/// and why it exists.
///
/// # Arguments
///
/// * `param1` - Description of parameter
/// * `param2` - Description of parameter
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// This function returns an error if:
/// - Condition 1 occurs
/// - Condition 2 occurs
///
/// # Panics
///
/// This function panics if... (only if it can panic)
///
/// # Safety
///
/// (Only for unsafe functions - document safety requirements)
///
/// # Examples
///
/// ```
/// use snow_owl_sftp::Server;
///
/// let server = Server::new(config).await?;
/// ```
///
/// # NIST 800-53: SC-13 (Cryptographic Protection)
/// # STIG: V-222577 - Applications must use cryptographic mechanisms
/// # Implementation: Uses russh library for SSH protocol encryption
pub async fn secure_operation(&self) -> Result<()> {
    // ...
}
```

#### Markdown Documentation Standards

- **Headers**: Use consistent hierarchy
- **Code Blocks**: Always specify language
- **Links**: Use reference-style for repeated links
- **Tables**: Align columns for readability
- **Lists**: Use consistent bullet style
- **Examples**: Include complete, runnable examples

### Version Control for Documentation

When committing:

```bash
# Good commit - includes documentation updates
git commit -m "feat(auth): add authorized_keys parsing

- Implement authorized_keys file parser
- Add key validation logic
- Update README.md with auth section
- Update RFC_COMPLIANCE.md for AC-2, IA-2
- Add NIST 800-53 controls to code
- Update ROADMAP.md Phase 1.1 progress"

# Bad commit - missing documentation
git commit -m "add auth stuff"  # ❌ NO!
```

### Documentation Review Checklist

Before committing:

```bash
# 1. Check all docs build without warnings
cargo doc --package snow-owl-sftp --no-deps

# 2. Review generated docs
cargo doc --package snow-owl-sftp --no-deps --open

# 3. Verify links work
# (Use a markdown link checker)

# 4. Check for typos
# (Use a spell checker)

# 5. Verify code examples compile
cargo test --doc --package snow-owl-sftp
```

---

## 🔄 Complete Workflow for ANY Change

### Step 1: Before Writing Code

1. Identify applicable NIST controls
2. Identify applicable STIG findings
3. Review existing documentation
4. Plan documentation updates

### Step 2: While Writing Code

1. Add NIST/STIG comments as you write
2. Write rustdoc for new public items
3. Add inline comments for complex logic
4. Consider security implications

### Step 3: After Writing Code

1. Run `cargo fmt`
2. Run `cargo clippy -- -D warnings`
3. Fix all issues (no exceptions without justification)
4. Add/update tests
5. Verify tests pass

### Step 4: Documentation Update

1. Update README.md (if applicable)
2. Update ROADMAP.md (mark tasks complete)
3. Update RFC_COMPLIANCE.md (if protocol/security changed)
4. Update QUICKSTART.md (if user-facing change)
5. Update config.example.toml (if config changed)
6. Add CHANGELOG.md entry
7. Update code documentation (rustdoc)

### Step 5: Pre-Commit Verification

Run the verification script:

```bash
#!/bin/bash
# verify.sh - Run before every commit

set -e

echo "=== Running pre-commit verification ==="

echo "1. Format check..."
cargo fmt --package snow-owl-sftp -- --check

echo "2. Clippy check..."
cargo clippy --package snow-owl-sftp -- -D warnings

echo "3. Build check..."
cargo build --package snow-owl-sftp

echo "4. Test check..."
cargo test --package snow-owl-sftp

echo "5. Doc check..."
cargo doc --package snow-owl-sftp --no-deps

echo "6. Doc test check..."
cargo test --doc --package snow-owl-sftp

echo "=== All checks passed! ==="
```

### Step 6: Commit

```bash
# Format: <type>(<scope>): <subject>
#
# <body with details>
# - Change 1
# - Change 2
# - Documentation updates
# - NIST controls: AC-3, SI-10
# - STIG: V-222596, V-222396
git commit -m "..."
```

---

## 📋 File-Specific Rules

### src/server.rs
- **REQUIRED**: AC-3, AC-12, AU-2, AU-3, IA-2, SC-8
- **STIG**: V-222596, V-222601, V-222602
- **Clippy**: Deny unwrap_used, expect_used

### src/client.rs
- **REQUIRED**: IA-2, SC-8, SC-13
- **STIG**: V-222577, V-222611
- **Clippy**: Deny unwrap_used, expect_used

### src/protocol.rs
- **REQUIRED**: SI-10, SC-13
- **STIG**: V-222396, V-222577
- **Clippy**: Allow complexity for protocol implementation

### src/config.rs
- **REQUIRED**: SI-10
- **STIG**: V-222396
- **Clippy**: Deny unwrap_used

### src/error.rs
- **REQUIRED**: SI-11, AU-3
- **STIG**: V-222566
- **Clippy**: All errors must implement std::error::Error

### src/lib.rs
- **REQUIRED**: Complete crate documentation
- **STIG**: Document security features
- **Clippy**: Deny missing_docs

---

## 🚨 Enforcement

### Automated Checks

Set up git hooks:

```bash
# .git/hooks/pre-commit
#!/bin/bash
./verify.sh || exit 1
```

### CI/CD Pipeline

All checks must pass in CI:

```yaml
# .github/workflows/sftp-checks.yml
- name: Format
  run: cargo fmt --package snow-owl-sftp -- --check

- name: Clippy
  run: cargo clippy --package snow-owl-sftp -- -D warnings

- name: Tests
  run: cargo test --package snow-owl-sftp

- name: Docs
  run: cargo doc --package snow-owl-sftp --no-deps
```

### Code Review Requirements

All PRs must:
- [ ] Pass all automated checks
- [ ] Include NIST/STIG comments
- [ ] Update relevant documentation
- [ ] Have reviewer verify compliance

### Rejection Criteria

PRs WILL BE REJECTED if:
- ❌ Missing NIST/STIG comments on security code
- ❌ Clippy warnings present
- ❌ Code not formatted with `cargo fmt`
- ❌ Documentation not updated
- ❌ Tests not added for new features
- ❌ Breaking changes without migration guide

---

## 📖 Examples

### Good Example

```rust
// NIST 800-53: AC-3 (Access Enforcement)
// STIG: V-222596 - The application must enforce approved authorizations
// Implementation: Validates all paths against root directory to prevent
// unauthorized file access through path traversal attacks
/// Resolves a client-provided path to a filesystem path.
///
/// This function ensures that the resolved path is within the configured
/// root directory, preventing path traversal attacks.
///
/// # Arguments
///
/// * `path` - The path provided by the client
///
/// # Returns
///
/// A `PathBuf` within the root directory
///
/// # Errors
///
/// Returns `Error::PermissionDenied` if the path attempts to traverse
/// outside the root directory.
///
/// # Examples
///
/// ```
/// # use snow_owl_sftp::*;
/// let session = SftpSession::new(config);
/// let safe_path = session.resolve_path("/etc/passwd")?;
/// // Returns: <root_dir>/etc/passwd
/// ```
fn resolve_path(&self, path: &str) -> Result<PathBuf> {
    let path = if path.starts_with('/') {
        &path[1..]
    } else {
        path
    };

    let resolved = self.config.root_dir.join(path);

    // NIST 800-53: AC-3 enforcement check
    if !resolved.starts_with(&self.config.root_dir) {
        return Err(Error::PermissionDenied(
            "Path traversal attempt".to_string(),
        ));
    }

    Ok(resolved)
}
```

### Bad Example

```rust
// ❌ NO NIST/STIG comments
// ❌ NO documentation
// ❌ Uses unwrap (clippy error)
fn resolve_path(&self, path: &str) -> PathBuf {
    self.config.root_dir.join(path).canonicalize().unwrap()
}
```

---

## 🎯 Success Criteria

You are following these rules correctly when:

1. ✅ Every commit has zero clippy warnings
2. ✅ Every commit is properly formatted
3. ✅ Every security function has NIST/STIG comments
4. ✅ Documentation is always current with code
5. ✅ All tests pass
6. ✅ Code reviews are smooth with no compliance issues

---

## 📞 Questions?

If unsure about:
- **NIST Controls**: Review NIST 800-53 Rev 5
- **STIG Requirements**: Review Application Security STIG
- **Clippy Rules**: Run `cargo clippy --help`
- **Documentation**: Review Rust documentation guidelines

---

**Last Updated**: 2026-01-19
**Version**: 1.0

These rules are MANDATORY and NON-NEGOTIABLE for all code in this crate.
