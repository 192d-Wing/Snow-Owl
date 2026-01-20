#!/bin/bash
# build.sh - Build script with proper PATH setup for Snow-Owl project

set -e

# Setup Rust environment
export PATH="$HOME/.cargo/bin:$PATH"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}Building Snow-Owl TFTP server and client...${NC}"

# Build release binaries (TFTP package only to avoid SFTP dependency issues)
cargo build --release -p snow-owl-tftp

echo -e "${GREEN}Build successful!${NC}"
echo ""
echo "Binaries created:"
ls -lh target/release/snow-owl-tftp-server
ls -lh target/release/snow-owl-tftp-client
echo ""
echo -e "${BLUE}To run tests:${NC}"
echo "  cd crates/snow-owl-tftp"
echo "  ../../run-tests.sh"
