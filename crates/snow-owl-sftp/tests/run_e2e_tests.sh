#!/bin/bash
# End-to-End Test Runner Script
#
# This script automates running end-to-end tests with a live SFTP server.
# It handles starting/stopping the server and running the test suite.
#
# NIST 800-53: CA-2 (Security Assessments), CA-8 (Penetration Testing)
# STIG: V-222648 (Security Testing)

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
SERVER_PORT=2222
SERVER_HOST="127.0.0.1"
TEST_DIR=$(mktemp -d)
SERVER_PID=""

# Cleanup function
cleanup() {
    echo -e "${YELLOW}Cleaning up...${NC}"

    if [ -n "$SERVER_PID" ]; then
        echo "Stopping server (PID: $SERVER_PID)"
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
    fi

    if [ -d "$TEST_DIR" ]; then
        echo "Removing test directory: $TEST_DIR"
        rm -rf "$TEST_DIR"
    fi

    echo -e "${GREEN}Cleanup complete${NC}"
}

# Register cleanup on exit
trap cleanup EXIT INT TERM

# Check prerequisites
check_prerequisites() {
    echo -e "${YELLOW}Checking prerequisites...${NC}"

    local missing=0

    if ! command -v sftp &> /dev/null; then
        echo -e "${RED}ERROR: 'sftp' command not found${NC}"
        echo "Please install OpenSSH client"
        missing=1
    else
        echo -e "${GREEN}✓${NC} sftp found: $(which sftp)"
    fi

    if ! command -v scp &> /dev/null; then
        echo -e "${RED}ERROR: 'scp' command not found${NC}"
        echo "Please install OpenSSH client"
        missing=1
    else
        echo -e "${GREEN}✓${NC} scp found: $(which scp)"
    fi

    if ! command -v ssh-keygen &> /dev/null; then
        echo -e "${RED}ERROR: 'ssh-keygen' command not found${NC}"
        echo "Please install OpenSSH client"
        missing=1
    else
        echo -e "${GREEN}✓${NC} ssh-keygen found: $(which ssh-keygen)"
    fi

    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}ERROR: 'cargo' command not found${NC}"
        echo "Please install Rust toolchain"
        missing=1
    else
        echo -e "${GREEN}✓${NC} cargo found: $(cargo --version)"
    fi

    if [ $missing -eq 1 ]; then
        echo -e "${RED}Missing prerequisites. Please install required tools.${NC}"
        exit 1
    fi

    echo -e "${GREEN}All prerequisites met!${NC}"
}

# Setup test environment
setup_environment() {
    echo -e "${YELLOW}Setting up test environment...${NC}"

    # Create directory structure
    mkdir -p "$TEST_DIR/server_root"
    mkdir -p "$TEST_DIR/client"
    mkdir -p "$TEST_DIR/keys"

    # Generate host key (Ed25519 for CNSA 2.0 compliance)
    echo "Generating host key..."
    ssh-keygen -t ed25519 -f "$TEST_DIR/keys/host_key" -N "" -C "test-host@localhost" >/dev/null 2>&1
    chmod 600 "$TEST_DIR/keys/host_key"

    # Generate client key (Ed25519 for CNSA 2.0 compliance)
    echo "Generating client key..."
    ssh-keygen -t ed25519 -f "$TEST_DIR/keys/client_key" -N "" -C "test-client@localhost" >/dev/null 2>&1
    chmod 600 "$TEST_DIR/keys/client_key"

    # Create authorized_keys
    cp "$TEST_DIR/keys/client_key.pub" "$TEST_DIR/keys/authorized_keys"
    chmod 600 "$TEST_DIR/keys/authorized_keys"

    # Create test files on server
    echo "Hello from server" > "$TEST_DIR/server_root/test_file.txt"
    echo "Large file content" | dd of="$TEST_DIR/server_root/large_file.bin" bs=1M count=10 2>/dev/null

    echo -e "${GREEN}Test environment ready at: $TEST_DIR${NC}"
}

# Start SFTP server
start_server() {
    echo -e "${YELLOW}Starting SFTP server...${NC}"

    # Build server if needed
    echo "Building server..."
    cargo build --bin snow-owl-sftp-server --release

    # Start server in background
    cargo run --bin snow-owl-sftp-server --release -- \
        --port $SERVER_PORT \
        --bind-address $SERVER_HOST \
        --root-dir "$TEST_DIR/server_root" \
        --host-key-path "$TEST_DIR/keys/host_key" \
        --authorized-keys-path "$TEST_DIR/keys/authorized_keys" \
        > "$TEST_DIR/server.log" 2>&1 &

    SERVER_PID=$!
    echo "Server started with PID: $SERVER_PID"

    # Wait for server to be ready
    echo "Waiting for server to be ready..."
    for i in {1..30}; do
        if nc -z $SERVER_HOST $SERVER_PORT 2>/dev/null; then
            echo -e "${GREEN}Server is ready!${NC}"
            return 0
        fi
        sleep 1
        echo -n "."
    done

    echo -e "${RED}ERROR: Server failed to start${NC}"
    echo "Server log:"
    cat "$TEST_DIR/server.log"
    return 1
}

# Run tests
run_tests() {
    echo -e "${YELLOW}Running end-to-end tests...${NC}"

    cd "$(dirname "$0")/.."

    # Run ignored tests
    cargo test --test e2e_client_tests -- --ignored --test-threads=1 || {
        echo -e "${RED}Tests failed!${NC}"
        echo "Server log:"
        cat "$TEST_DIR/server.log"
        return 1
    }

    echo -e "${GREEN}All tests passed!${NC}"
}

# Main execution
main() {
    echo -e "${GREEN}=== Snow Owl SFTP End-to-End Test Runner ===${NC}"
    echo ""

    check_prerequisites
    echo ""

    setup_environment
    echo ""

    if start_server; then
        echo ""
        run_tests
    else
        echo -e "${RED}Failed to start server${NC}"
        exit 1
    fi

    echo ""
    echo -e "${GREEN}=== Test run complete ===${NC}"
}

# Run main function
main
