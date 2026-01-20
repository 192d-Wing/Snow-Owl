# Build Release Fix - crypto-primes Dependency Issue

## Problem

Running `cargo build --release` from the project root fails with compilation errors in the `crypto-primes` dependency.

## Root Cause Analysis

### The Issue
The `crypto-primes v0.7.0-pre.6` crate has compilation errors with 17 type annotation and trait implementation issues.

### Dependency Chain
```
crypto-primes v0.7.0-pre.6
└── rsa v0.10.0-rc.12
    └── russh v0.56.0
        └── snow-owl-sftp v0.1.0
```

**Key Finding:** The problem is in the **SFTP crate**, not the TFTP crate!

### Why It Fails

When running `cargo build --release` from the workspace root, Cargo tries to build **ALL** workspace members:
- ✅ `snow-owl-tftp` - No issues, builds fine
- ❌ `snow-owl-sftp` - Has transitive dependency on broken `crypto-primes`

## Solution

### Option 1: Build Only TFTP Package (Recommended)

```bash
# Instead of:
cargo build --release

# Use:
cargo build --release -p snow-owl-tftp
```

This builds only the TFTP package and avoids the SFTP dependency issues.

### Option 2: Fix SFTP Dependencies

Update the SFTP crate's dependencies to use compatible versions:

```bash
cd crates/snow-owl-sftp
cargo update
```

However, this may require code changes if the API changed.

## Implementation

### Files Updated

#### 1. `build.sh`
```bash
# Before
cargo build --release

# After
cargo build --release -p snow-owl-tftp
```

#### 2. `crates/snow-owl-tftp/tests/run-all-tests.sh`
```bash
# Before
cargo build --release

# After
cargo build --release -p snow-owl-tftp
```

## Verification

### Test Build Script
```bash
./build.sh
```

**Expected output:**
```
Building Snow-Owl TFTP server and client...
Finished `release` profile [optimized] target(s) in 0.06s
Build successful!

Binaries created:
-rwxrwxr-x 2 jwillman jwillman 4.8M target/release/snow-owl-tftp-server
-rwxrwxr-x 2 jwillman jwillman 2.9M target/release/snow-owl-tftp-client
```

### Test Integration Tests
```bash
./run-tests.sh -i
```

**Expected:** All 10 integration tests pass

## Technical Details

### Why TFTP Builds Fine

The TFTP crate dependencies:
```toml
[dependencies]
tokio = { ... }
bytes = { ... }
anyhow = { ... }
# ... no russh, no rsa, no crypto-primes
```

No problematic dependencies!

### Why SFTP Has Issues

The SFTP crate dependencies include:
```
russh → rsa → crypto-primes (broken)
```

The `crypto-primes v0.7.0-pre.6` is a pre-release version with compilation issues on recent Rust versions.

### Error Details

The `crypto-primes` errors are all related to:
1. **Type annotations needed** - Generic type inference failures
2. **Mismatched types** - Type parameter vs integer mismatches
3. **Trait bounds not satisfied** - Missing `RngCore` implementation

These are likely due to breaking changes in upstream dependencies like `crypto-bigint`.

## Workspace Build Behavior

### Default Behavior
```bash
cargo build --release
```
Builds **all** workspace members listed in root `Cargo.toml`:
- `crates/snow-owl-tftp` ✅
- `crates/snow-owl-sftp` ❌

### Package-Specific Build
```bash
cargo build --release -p snow-owl-tftp
```
Builds **only** the specified package and its dependencies.

## Long-Term Solutions

### For SFTP Crate

1. **Update russh dependency** to a stable version without `crypto-primes` issues
2. **Pin dependency versions** to known-working combinations
3. **Consider alternatives** to `russh` if updates don't fix it

Example fix attempt:
```toml
[dependencies]
# Try updating to a newer russh version
russh = "0.57"  # Or latest stable

# Or pin specific sub-dependencies
[dependencies.rsa]
version = "0.9"  # Use stable version instead of RC
```

### For Build Scripts

Already implemented - use `-p snow-owl-tftp` flag to build only TFTP.

## Current Status

| Component | Status | Command |
|-----------|--------|---------|
| TFTP Build | ✅ Working | `cargo build --release -p snow-owl-tftp` |
| TFTP Tests | ✅ Passing | `./run-tests.sh` |
| SFTP Build | ❌ Broken | `cargo-primes` dependency issue |
| Workspace Build | ❌ Fails | Due to SFTP issues |

## Workarounds

### For TFTP Development

Always use:
```bash
# Build
./build.sh
# OR
cargo build --release -p snow-owl-tftp

# Test
./run-tests.sh
```

### For SFTP Development

Need to fix the dependency issue first:
```bash
cd crates/snow-owl-sftp
cargo update
# If that doesn't work, may need to update russh version in Cargo.toml
```

## Error Example

When running `cargo build --release` from workspace root:

```
error[E0282]: type annotations needed
   --> .../crypto-primes-0.7.0-pre.6/src/fips.rs:143:18
    |
143 |     if candidate.as_ref()[0].0 & 3 != 3 {
    |                  ^^^^^^

error[E0308]: mismatched types
   --> .../crypto-primes-0.7.0-pre.6/src/fips.rs:143:39
    |
143 |     if candidate.as_ref()[0].0 & 3 != 3 {
    |                                       ^ expected type parameter `T`, found integer

... (17 errors total)

error: could not compile `crypto-primes` (lib) due to 17 previous errors
```

## Summary

✅ **Fixed:** TFTP build works with `-p snow-owl-tftp` flag
❌ **Issue:** SFTP crate has dependency problems
✅ **Workaround:** Build only TFTP package
📋 **TODO:** Fix SFTP dependencies separately

## Quick Commands

```bash
# Build TFTP only (works)
./build.sh
cargo build --release -p snow-owl-tftp

# Build everything (fails due to SFTP)
cargo build --release  # ❌ Don't use this

# Test TFTP
./run-tests.sh  # ✅ Works fine

# Check TFTP dependencies
cd crates/snow-owl-tftp
cargo tree
```

## Related Documentation

- [BUILD_AND_TEST.md](BUILD_AND_TEST.md) - Build and test guide
- [BUILD_FIX_SUMMARY.md](BUILD_FIX_SUMMARY.md) - Initial build fix
- [INTEGRATION_TEST_FIX.md](INTEGRATION_TEST_FIX.md) - Test fixes
- [QUICK_REFERENCE.md](QUICK_REFERENCE.md) - Quick reference
