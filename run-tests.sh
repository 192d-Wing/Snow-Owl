#!/bin/bash
# run-tests.sh - Test runner with proper PATH setup

set -e

# Setup Rust environment
export PATH="$HOME/.cargo/bin:$PATH"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}Snow-Owl TFTP Test Runner${NC}"
echo ""

# Kill any existing TFTP servers
pkill -f snow-owl-tftp-server 2>/dev/null || true
sleep 1

# Check if built
if [ ! -f "target/release/snow-owl-tftp-server" ]; then
    echo -e "${YELLOW}Binaries not found. Building first...${NC}"
    ./build.sh
fi

# Verify binary exists
if [ ! -f "target/release/snow-owl-tftp-server" ]; then
    echo -e "${RED}ERROR: Build failed or binary not found${NC}"
    exit 1
fi

echo -e "${GREEN}✓ TFTP server binary found${NC}"
echo ""

# Navigate to TFTP crate tests directory
cd crates/snow-owl-tftp/tests

# Run tests
echo -e "${BLUE}Running test suite...${NC}"
./run-all-tests.sh "$@"
