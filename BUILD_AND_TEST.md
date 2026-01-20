# Snow-Owl Build and Test Guide

## Quick Start

### Build Everything

```bash
./build.sh
```

This will:
- Set up the Rust environment
- Build all binaries in release mode
- Display binary locations

### Run Tests

```bash
./run-tests.sh
```

This will:
- Build if needed
- Run integration tests
- Run windowsize tests (if atftp is available)
- Display comprehensive results

## Build Issues Fixed

### Issue: cargo build --release fails

**Problem:** `crypto-primes` dependency had compilation errors

**Solution:** Run `cargo update` to update dependencies

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo update
cargo build --release
```

**Status:** ✅ FIXED

The issue was caused by outdated dependencies. Running `cargo update` updated `zmij` crate which resolved the transitive dependency conflicts.

## Manual Build Steps

If you need to build manually:

```bash
# Setup environment
export PATH="$HOME/.cargo/bin:$PATH"

# Build release binaries
cargo build --release

# Build specific binary
cargo build --release --bin snow-owl-tftp-server
cargo build --release --bin snow-owl-tftp-client

# Build debug binaries (faster, but not optimized)
cargo build
```

## Binary Locations

After building, binaries are located at:

```
target/release/snow-owl-tftp-server  # 4.9 MB
target/release/snow-owl-tftp-client  # 3.0 MB
```

## Test Requirements

### For Integration Tests
```bash
# Ubuntu/Debian
sudo apt-get install tftp-hpa

# Verify
tftp --version
```

### For Windowsize Tests
```bash
# Ubuntu/Debian
sudo apt-get install atftp

# Verify
atftp --version
```

### For Performance Analysis
```bash
# Python 3.6+ (usually pre-installed)
python3 --version
```

## Running Specific Test Suites

### Integration Tests Only
```bash
cd crates/snow-owl-tftp
./tests/integration-test.sh
```

### Windowsize Tests Only
```bash
cd crates/snow-owl-tftp
./tests/windowsize-test.sh
```

### Python Performance Analyzer
```bash
cd crates/snow-owl-tftp

# Quick test
./tests/windowsize-analyzer.py quick

# Full suite (32 tests)
./tests/windowsize-analyzer.py full

# Performance comparison
./tests/windowsize-analyzer.py performance
```

### All Tests with Options
```bash
./run-tests.sh              # Run all tests
./run-tests.sh -i           # Integration only
./run-tests.sh -w           # Windowsize only
./run-tests.sh -p           # Include performance analysis
./run-tests.sh --skip-build # Skip building
```

## Troubleshooting

### cargo: command not found

**Solution:** Add cargo to PATH

```bash
export PATH="$HOME/.cargo/bin:$PATH"

# Make permanent (add to ~/.bashrc or ~/.zshrc)
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### Build fails with dependency errors

**Solution:** Update dependencies

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo update
cargo build --release
```

### Server binary not found during tests

**Solution:** Build from project root

```bash
cd /home/jwillman/projects/snow-owl
cargo build --release

# Verify binaries exist
ls -lh target/release/snow-owl-tftp-*
```

### atftp not found

**Solution:** Install atftp or use Python analyzer instead

```bash
# Option 1: Install atftp
sudo apt-get install atftp

# Option 2: Use Python analyzer (no atftp needed)
cd crates/snow-owl-tftp
./tests/windowsize-analyzer.py performance
```

### Permission denied on test scripts

**Solution:** Make scripts executable

```bash
chmod +x build.sh
chmod +x run-tests.sh
chmod +x crates/snow-owl-tftp/tests/*.sh
chmod +x crates/snow-owl-tftp/tests/*.py
```

### Port already in use

**Solution:** Change test port or kill existing process

```bash
# Find process using port 6970
sudo netstat -tulpn | grep 6970

# Kill process if needed
kill <PID>
```

## Build Artifacts

### Release Build
- **Size:** ~5-8 MB per binary
- **Optimization:** Full
- **Debug symbols:** Stripped
- **Performance:** Production-ready

### Debug Build
- **Size:** ~15-20 MB per binary
- **Optimization:** Minimal
- **Debug symbols:** Included
- **Performance:** Slower, but easier to debug

## Environment Variables

```bash
# Rust environment
export PATH="$HOME/.cargo/bin:$PATH"

# Rust toolchain (optional)
export RUSTUP_HOME="$HOME/.rustup"
export CARGO_HOME="$HOME/.cargo"

# Logging (optional)
export RUST_LOG=debug  # Enable debug logging
export RUST_BACKTRACE=1  # Enable backtraces on panic
```

## CI/CD Integration

For automated builds:

```bash
#!/bin/bash
set -e

# Setup environment
export PATH="$HOME/.cargo/bin:$PATH"

# Update dependencies
cargo update

# Build
cargo build --release

# Test
cargo test

# Run integration tests (requires tftp client)
cd crates/snow-owl-tftp
./tests/integration-test.sh
```

## Performance Notes

### Build Times
- **Clean release build:** ~5-10 minutes (first time)
- **Incremental build:** ~5-30 seconds
- **Clean debug build:** ~2-5 minutes

### Optimization Tips
1. Use `cargo build --release` for production
2. Use `cargo build` for development (faster iteration)
3. Use `cargo check` for syntax checking (no binary)
4. Use `sccache` or `mold` linker for faster builds

## Next Steps

After successful build:

1. ✅ Build completed successfully
2. 📋 Run tests to validate implementation
3. 🚀 Deploy to production environment
4. 📊 Monitor performance and logs

## Documentation

- [TESTING_INSTRUCTIONS.md](crates/snow-owl-tftp/tests/TESTING_INSTRUCTIONS.md) - Detailed test documentation
- [WINDOWSIZE_TESTS.md](crates/snow-owl-tftp/tests/WINDOWSIZE_TESTS.md) - RFC 7440 windowsize tests
- [README.md](crates/snow-owl-tftp/tests/README.md) - Test suite overview

## Support

For build or test issues:
1. Check this guide's troubleshooting section
2. Verify Rust toolchain is installed: `cargo --version`
3. Update dependencies: `cargo update`
4. Clean and rebuild: `cargo clean && cargo build --release`
