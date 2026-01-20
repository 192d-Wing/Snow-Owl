# Development Rules Summary

**4 MANDATORY RULES FOR EVERY ACTION**

---

## Rule 1: Security Compliance Documentation ✅

**ALL code MUST include NIST Controls and Application Development STIG comments**

### What to Add

```rust
// NIST 800-53: AC-3 (Access Enforcement)
// STIG: V-222596 - The application must enforce approved authorizations
// Implementation: Path traversal protection ensures users cannot access
// files outside their authorized root directory
fn resolve_path(&self, path: &str) -> Result<PathBuf> {
    // ... code ...
}
```

### When to Add

- **ALWAYS** for authentication code (AC-2, IA-2)
- **ALWAYS** for authorization checks (AC-3)
- **ALWAYS** for input validation (SI-10)
- **ALWAYS** for error handling (SI-11)
- **ALWAYS** for audit/logging (AU-2, AU-3)
- **ALWAYS** for crypto operations (SC-13)
- **ALWAYS** for session management (AC-11, AC-12)

### Key Controls to Know

| Code Type | NIST Control | STIG ID |
|-----------|--------------|---------|
| Path validation | AC-3 | V-222596 |
| Input validation | SI-10 | V-222396 |
| Authentication | IA-2 | V-222611 |
| Error handling | SI-11 | V-222566 |
| Encryption | SC-13 | V-222577 |
| Session timeout | AC-12 | V-222601 |

**See**: [DEVELOPMENT_RULES.md](DEVELOPMENT_RULES.md) for complete list

---

## Rule 2: IPv6 Network Support ✅

**ALL network code MUST support IPv6 and prefer IPv6 by default when available**

### Requirements

```rust
// NIST 800-53: SC-7 (Boundary Protection), SC-8 (Transmission Confidentiality)
// Implementation: IPv6 support with dual-stack capability
/// Bind server to address with IPv6 preference
pub async fn bind(config: &Config) -> Result<TcpListener> {
    // Try IPv6 dual-stack first
    match TcpListener::bind(format!("[::]:{}", config.port)).await {
        Ok(listener) => {
            info!("Bound to IPv6 dual-stack [::]:{}", config.port);
            Ok(listener)
        }
        Err(_) => {
            // Fallback to IPv4
            warn!("IPv6 unavailable, using IPv4");
            TcpListener::bind(format!("0.0.0.0:{}", config.port)).await
        }
    }
}
```

### Checklist for Network Code

- [ ] Supports IPv6 addresses
- [ ] Prefers IPv6 when available
- [ ] Falls back to IPv4 gracefully
- [ ] Configuration allows IPv6-only mode
- [ ] Tested with both IPv4 and IPv6
- [ ] IP parsing handles both formats
- [ ] Rate limiting works with both families
- [ ] NIST SC-7 documented

**See**: [DEVELOPMENT_RULES.md](DEVELOPMENT_RULES.md) Section 2

---

## Rule 3: Code Quality Standards ✅

**ALL code MUST pass `cargo fmt` and `cargo clippy` with ZERO issues**

### Before Every Commit

```bash
# 1. Format code (REQUIRED)
cargo fmt --package snow-owl-sftp

# 2. Check formatting (MUST pass)
cargo fmt --package snow-owl-sftp -- --check

# 3. Run clippy (MUST have no warnings)
cargo clippy --package snow-owl-sftp -- -D warnings

# 4. Run the verification script
cd crates/snow-owl-sftp && ./verify.sh
```

### What's Enforced

- ❌ **NO** `unwrap()` or `expect()` - Use `?` instead
- ❌ **NO** `panic!()` - Return errors properly
- ❌ **NO** `todo!()` - Complete implementation
- ❌ **NO** `unsafe` without documentation
- ✅ **YES** Complete rustdoc for public items
- ✅ **YES** Error handling with `Result<T>`
- ✅ **YES** Formatted with `cargo fmt`

### Quick Fix

```bash
# Auto-fix most issues
cargo clippy --package snow-owl-sftp --fix

# Format code
cargo fmt --package snow-owl-sftp
```

**See**: [DEVELOPMENT_RULES.md](DEVELOPMENT_RULES.md) Section 3

---

## Rule 4: Documentation Synchronization ✅

**ALL documentation MUST be updated at the end of EVERY action**

### What to Update

**Every Commit:**
- [ ] Code comments (NIST/STIG)
- [ ] Rustdoc for public items
- [ ] CHANGELOG.md

**When Adding Features:**
- [ ] README.md (feature list)
- [ ] ROADMAP.md (mark complete)
- [ ] QUICKSTART.md (usage example)

**When Changing Config:**
- [ ] config.example.toml
- [ ] QUICKSTART.md
- [ ] README.md

**When Changing Protocol:**
- [ ] RFC_COMPLIANCE.md
- [ ] protocol.rs docs

**When Changing Security:**
- [ ] SECURITY.md
- [ ] RFC_COMPLIANCE.md
- [ ] README.md

### Documentation Checklist

```bash
# 1. Build docs (must succeed)
cargo doc --package snow-owl-sftp --no-deps

# 2. Check doc tests
cargo test --doc --package snow-owl-sftp

# 3. Verify CHANGELOG entry added
git diff CHANGELOG.md

# 4. Verify relevant docs updated
git status
```

**See**: [DEVELOPMENT_RULES.md](DEVELOPMENT_RULES.md) Section 4

---

## Complete Workflow

### Step 1: Before Writing Code
1. Read NIST controls for your code area
2. Review STIG requirements
3. Plan documentation updates
4. Plan IPv6 support for network code

### Step 2: While Writing Code
1. Add NIST/STIG comments as you write
2. Implement IPv6 support for network operations
3. Write rustdoc for new public items
4. Use `Result<T>` for error handling
5. No unwrap/expect/panic

### Step 3: After Writing Code
1. `cargo fmt --package snow-owl-sftp`
2. `cargo clippy --package snow-owl-sftp -- -D warnings`
3. Fix all issues
4. Add/update tests (including IPv6 tests for network code)
5. Verify tests pass

### Step 4: Update Documentation
1. Update CHANGELOG.md
2. Update README.md (if applicable)
3. Update ROADMAP.md (mark tasks complete)
4. Update RFC_COMPLIANCE.md (if protocol changed)
5. Update SECURITY.md (if security changed)
6. Update config.example.toml (if config changed)
7. Update QUICKSTART.md (if user-facing)

### Step 5: Verify
```bash
cd crates/snow-owl-sftp
./verify.sh
```

### Step 6: Commit
```bash
git commit -m "feat(scope): description

- Change 1
- Change 2
- Documentation: CHANGELOG.md, README.md
- NIST controls: AC-3, SI-10
- STIG: V-222596, V-222396

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Quick Reference

### Essential Commands

```bash
# Format
cargo fmt --package snow-owl-sftp

# Lint
cargo clippy --package snow-owl-sftp -- -D warnings

# Test
cargo test --package snow-owl-sftp

# Verify all
./verify.sh
```

### Essential Files

| File | Purpose |
|------|---------|
| DEVELOPMENT_RULES.md | Complete rules (READ THIS) |
| QUICK_REFERENCE.md | One-page cheat sheet |
| SECURITY.md | NIST/STIG details |
| verify.sh | Pre-commit verification |

### Essential Controls

| Control | Use For |
|---------|---------|
| AC-3 | Access checks, path validation |
| SI-10 | Input validation |
| SI-11 | Error handling |
| IA-2 | Authentication |
| SC-13 | Encryption |

### Essential STIGs

| STIG | Use For |
|------|---------|
| V-222396 | Input validation |
| V-222566 | Error messages |
| V-222577 | Crypto |
| V-222596 | Authorization |

---

## Enforcement

### Automated

- Git pre-commit hook (optional)
- CI/CD pipeline checks
- `verify.sh` script

### Manual

- Code review requirements
- PR template checklist
- Reviewer verification

### Rejection Criteria

PRs **WILL BE REJECTED** if:
- ❌ Missing NIST/STIG comments
- ❌ Clippy warnings present
- ❌ Not formatted
- ❌ Documentation not updated
- ❌ Tests missing

---

## Examples

### ✅ GOOD

```rust
// NIST 800-53: AC-3 (Access Enforcement)
// STIG: V-222596 - Authorization enforcement
// Implementation: Validates path within root directory
/// Resolves a client path to filesystem path.
///
/// # Arguments
/// * `path` - Client-provided path
///
/// # Returns
/// Resolved `PathBuf` within root directory
///
/// # Errors
/// Returns `Error::PermissionDenied` if path traversal detected
fn resolve_path(&self, path: &str) -> Result<PathBuf> {
    let resolved = self.root.join(path);

    if !resolved.starts_with(&self.root) {
        return Err(Error::PermissionDenied("Path traversal".into()));
    }

    Ok(resolved)
}
```

### ❌ BAD

```rust
// NO NIST/STIG comments
// NO documentation
fn resolve_path(&self, path: &str) -> PathBuf {
    self.root.join(path).canonicalize().unwrap() // Uses unwrap!
}
```

---

## Resources

### Documentation
- [DEVELOPMENT_RULES.md](DEVELOPMENT_RULES.md) - Full rules
- [QUICK_REFERENCE.md](QUICK_REFERENCE.md) - Cheat sheet
- [SECURITY.md](SECURITY.md) - Security compliance

### External
- [NIST 800-53](https://csrc.nist.gov/publications/detail/sp/800-53/rev-5/final)
- [Application STIG](https://public.cyber.mil/stigs/)
- [Rust Clippy](https://rust-lang.github.io/rust-clippy/master/)

---

## Questions?

1. **What NIST control do I use?** → See SECURITY.md Section "NIST 800-53 Controls"
2. **What STIG ID applies?** → See SECURITY.md Section "Application Security STIG"
3. **How do I fix clippy warnings?** → Run `cargo clippy --fix`
4. **What documentation to update?** → See Rule 3 checklist
5. **Why is this so strict?** → Security and compliance requirements

---

## Summary

**4 Simple Rules:**

1. **Add NIST/STIG comments** to security code
2. **Support IPv6** for all network code (prefer IPv6, fallback to IPv4)
3. **Pass clippy and fmt** before committing
4. **Update docs** with every change

**Run before every commit:**
```bash
./verify.sh
```

**That's it!** Follow these rules and your code will be compliant, secure, and maintainable.

---

**Last Updated**: 2026-01-19
**Print this and keep it visible while coding!**
