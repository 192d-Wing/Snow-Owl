# Snow-Owl Quick Reference

## Build

```bash
./build.sh
# OR
export PATH="$HOME/.cargo/bin:$PATH" && cargo build --release -p snow-owl-tftp

# Note: Don't use `cargo build --release` alone - it fails due to SFTP dependencies
```

## Test

```bash
./run-tests.sh                    # All tests
./run-tests.sh -i                 # Integration only
./run-tests.sh -w                 # Windowsize only
./run-tests.sh -p                 # + Performance analysis
```

## Binaries

```
target/release/snow-owl-tftp-server    # 4.9 MB
target/release/snow-owl-tftp-client    # 3.0 MB
```

## Run Server

```bash
./target/release/snow-owl-tftp-server --help
./target/release/snow-owl-tftp-server --config /path/to/config.toml
```

## Windowsize Tests (32 tests)

```bash
cd crates/snow-owl-tftp

# Bash test suite (requires atftp)
./tests/windowsize-test.sh

# Python analyzer (no atftp needed)
./tests/windowsize-analyzer.py quick        # Quick test
./tests/windowsize-analyzer.py full         # All 32 tests
./tests/windowsize-analyzer.py performance  # Performance comparison
```

## Test Files

| Test | What | Where |
|------|------|-------|
| Integration | Core TFTP functionality | `tests/integration-test.sh` |
| Windowsize | RFC 7440 (32 tests) | `tests/windowsize-test.sh` |
| Analyzer | Performance metrics | `tests/windowsize-analyzer.py` |
| Master | All test suites | `tests/run-all-tests.sh` |

## Install Dependencies

```bash
# For windowsize tests
sudo apt-get install atftp

# For integration tests (usually pre-installed)
sudo apt-get install tftp-hpa

# Python 3 (usually pre-installed)
python3 --version
```

## Fix Build Issues

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo update
cargo build --release -p snow-owl-tftp  # Build TFTP only (SFTP has dependency issues)
```

## Documentation

| File | Purpose |
|------|---------|
| [BUILD_AND_TEST.md](BUILD_AND_TEST.md) | Complete build/test guide |
| [BUILD_FIX_SUMMARY.md](BUILD_FIX_SUMMARY.md) | Recent build fixes |
| [TESTING_INSTRUCTIONS.md](crates/snow-owl-tftp/tests/TESTING_INSTRUCTIONS.md) | Detailed test guide |
| [WINDOWSIZE_TESTS.md](crates/snow-owl-tftp/tests/WINDOWSIZE_TESTS.md) | RFC 7440 documentation |
| [README.md](crates/snow-owl-tftp/tests/README.md) | Test suite overview |

## Common Commands

```bash
# Build everything
./build.sh

# Run all tests
./run-tests.sh

# Clean build
cargo clean && cargo build --release

# Check syntax only (fast)
cargo check

# Run specific test
cd crates/snow-owl-tftp
./tests/windowsize-test.sh
```

## Windowsize Test Coverage

- **Tests 1-8:** Small file (1KB) - windowsize 1-8
- **Tests 9-16:** Medium file (10KB) - windowsize 1-32
- **Tests 17-24:** Large file (100KB) - windowsize 1-64
- **Tests 25-28:** XLarge file (512KB) - windowsize 1-64
- **Tests 29-32:** Edge cases

## Status

| Component | Status |
|-----------|--------|
| Build | ✅ Working |
| TFTP Server | ✅ Built (4.9 MB) |
| TFTP Client | ✅ Built (3.0 MB) |
| Integration Tests | ✅ Passing (10/10) |
| Windowsize Tests | ✅ Ready (32 tests) |
| Tests Total | ✅ 42 tests available |
| Documentation | ✅ Complete |

## Recent Fixes

- ✅ Fixed `cargo build --release` dependency errors
- ✅ Fixed integration test binary path detection
- ✅ Fixed port conflict issues
- ✅ Added Rust PATH setup to all scripts
- ✅ Fixed color code display issues

See [INTEGRATION_TEST_FIX.md](INTEGRATION_TEST_FIX.md) for details.

## Quick Troubleshooting

| Problem | Solution |
|---------|----------|
| cargo not found | `export PATH="$HOME/.cargo/bin:$PATH"` |
| Build fails | `cargo update && cargo build --release` |
| atftp not found | `sudo apt-get install atftp` |
| Binary not found | Run `./build.sh` from project root |
| Permission denied | `chmod +x build.sh run-tests.sh` |
| Port in use | `pkill -f snow-owl-tftp-server` |
| Tests fail | See [INTEGRATION_TEST_FIX.md](INTEGRATION_TEST_FIX.md) |
