# Quick Reference Card

**One-page reference for Snow Owl SFTP development**

---

## 🚀 Quick Start

```bash
# Build
cargo build --package snow-owl-sftp

# Run server
cargo run --bin snow-owl-sftp-server -- --root /tmp/sftp --verbose

# Run tests
cargo test --package snow-owl-sftp

# Run verification (REQUIRED before commit)
./verify.sh
```

---

## 📋 Pre-Commit Checklist

**MUST complete before EVERY commit:**

```bash
# 1. Format code
cargo fmt --package snow-owl-sftp

# 2. Fix clippy warnings
cargo clippy --package snow-owl-sftp -- -D warnings

# 3. Run tests
cargo test --package snow-owl-sftp

# 4. Verify everything
cd crates/snow-owl-sftp && ./verify.sh
```

---

## 🔒 Required Comment Template

**For ALL security-relevant code:**

```rust
// NIST 800-53: [Control ID] ([Control Name])
// STIG: [V-ID] - [Requirement Description]
// Implementation: [How this code satisfies the requirement]
/// Rustdoc description
///
/// # NIST 800-53: [Control ID]
/// # STIG: [V-ID]
/// # Implementation: [Details]
pub fn security_function() -> Result<()> {
    // ...
}
```

---

## 📚 Common NIST Controls

| Control | Name | Use For |
|---------|------|---------|
| AC-2 | Account Management | User management, authorized_keys |
| AC-3 | Access Enforcement | Path validation, permissions |
| AC-7 | Unsuccessful Logon | Auth rate limiting |
| AC-11 | Session Lock | Timeouts |
| AC-12 | Session Termination | Cleanup |
| AU-2 | Audit Events | Logging what happened |
| AU-3 | Audit Content | What to log (who, when, what) |
| IA-2 | Authentication | SSH key auth |
| SC-7 | Boundary Protection | Network protocols, IPv6 |
| SC-8 | Transmission Confidentiality | Encryption |
| SC-13 | Cryptographic Protection | Crypto algorithms |
| SI-10 | Input Validation | All user input |
| SI-11 | Error Handling | Error messages |

---

## 🛡️ Common STIG IDs

| STIG ID | Requirement | Apply To |
|---------|-------------|----------|
| V-222396 | Input validation | All user inputs |
| V-222566 | Secure error messages | Error handling |
| V-222577 | Cryptographic mechanisms | Encryption code |
| V-222596 | Authorization enforcement | Access checks |
| V-222601 | Session termination | Timeout logic |
| V-222602 | Session ID protection | Session handling |
| V-222611 | Certificate validation | Key verification |

---

## 🔧 File-Specific Requirements

| File | Required Controls | Required STIGs |
|------|------------------|----------------|
| server.rs | AC-3, AC-12, AU-2, IA-2, SC-8 | V-222596, V-222601, V-222602 |
| client.rs | IA-2, SC-8, SC-13 | V-222577, V-222611 |
| protocol.rs | SI-10, SC-13 | V-222396, V-222577 |
| config.rs | SI-10 | V-222396 |
| error.rs | SI-11, AU-3 | V-222566 |

---

## 📖 Documentation Update Rules

**Always update when:**
- ✅ Adding features → README.md, ROADMAP.md
- ✅ Changing config → config.example.toml, QUICKSTART.md
- ✅ Protocol changes → RFC_COMPLIANCE.md
- ✅ Security changes → SECURITY.md
- ✅ Bug fixes → CHANGELOG.md
- ✅ ANY commit → CHANGELOG.md

---

## 🚫 Never Allow

```rust
// ❌ NO unwrap
let value = something.unwrap();

// ❌ NO expect
let value = something.expect("failed");

// ❌ NO panic
panic!("error");

// ❌ NO todo
todo!();

// ❌ NO missing NIST comments on security code
fn authorize() { ... }

// ❌ NO unsafe without documentation
unsafe { ... }
```

---

## ✅ Always Do

```rust
// ✅ YES Result and ?
let value = something?;

// ✅ YES NIST/STIG comments
// NIST 800-53: AC-3
// STIG: V-222596
fn authorize() { ... }

// ✅ YES IPv6 support for network code
// NIST 800-53: SC-7
match TcpListener::bind(format!("[::]:{}", port)).await {
    Ok(listener) => Ok(listener), // IPv6 dual-stack
    Err(_) => TcpListener::bind(format!("0.0.0.0:{}", port)).await, // IPv4 fallback
}

// ✅ YES complete rustdoc
/// Function description
///
/// # Arguments
/// # Returns
/// # Errors
pub fn function() { ... }

// ✅ YES update CHANGELOG
# Added new auth feature (AC-2, V-222611)
```

---

## 🎯 Commit Message Format

```
<type>(<scope>): <subject>

<body>
- Change 1
- Change 2
- Documentation: [files updated]
- NIST controls: [list]
- STIG: [list]

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```

**Types:** feat, fix, docs, style, refactor, test, chore

---

## 🔍 Quick Checks

```bash
# Check format
cargo fmt --package snow-owl-sftp -- --check

# Check clippy
cargo clippy --package snow-owl-sftp -- -D warnings

# Build docs
cargo doc --package snow-owl-sftp --no-deps --open

# Run specific test
cargo test --package snow-owl-sftp test_name

# Check dependencies
cargo tree --package snow-owl-sftp
```

---

## 📁 Where to Find Things

| What | Where |
|------|-------|
| Development rules | DEVELOPMENT_RULES.md |
| Security policy | SECURITY.md |
| RFC compliance | RFC_COMPLIANCE.md |
| Quick start | QUICKSTART.md |
| Roadmap | ROADMAP.md |
| Changes | CHANGELOG.md |
| Verify script | verify.sh |

---

## 🆘 Common Issues

### Clippy Warnings

```bash
# See all warnings
cargo clippy --package snow-owl-sftp

# Fix most automatically
cargo clippy --package snow-owl-sftp --fix
```

### Format Issues

```bash
# Fix formatting
cargo fmt --package snow-owl-sftp

# Check formatting
cargo fmt --package snow-owl-sftp -- --check
```

### Missing Docs

```bash
# Find missing docs
cargo doc --package snow-owl-sftp --no-deps 2>&1 | grep warning
```

### Test Failures

```bash
# Verbose test output
cargo test --package snow-owl-sftp -- --nocapture

# Run specific test
cargo test --package snow-owl-sftp test_name -- --nocapture
```

---

## 🔗 External References

- [NIST 800-53 Rev 5](https://csrc.nist.gov/publications/detail/sp/800-53/rev-5/final)
- [Application STIG](https://public.cyber.mil/stigs/)
- [Rust Clippy Lints](https://rust-lang.github.io/rust-clippy/master/)
- [RFC 4251-4254](https://datatracker.ietf.org/doc/html/rfc4251)

---

## 💡 Pro Tips

1. **Run verify.sh before committing** - Saves time in CI
2. **Add NIST comments as you code** - Easier than retroactively
3. **Support IPv6 for all network code** - Prefer `[::]` over `0.0.0.0`
4. **Update docs immediately** - Don't let them get stale
5. **Use descriptive commit messages** - Helps reviewers
6. **Run clippy often** - Fix issues early
7. **Keep CHANGELOG current** - Every commit should have entry
8. **Read DEVELOPMENT_RULES.md** - Know the requirements

---

**Print this page and keep it handy while developing!**

Last Updated: 2026-01-20
