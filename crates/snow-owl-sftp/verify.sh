#!/bin/bash
# Snow Owl SFTP Pre-Commit Verification Script
# This script MUST pass before any commit

set -e

PACKAGE="snow-owl-sftp"
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================"
echo "Snow Owl SFTP Pre-Commit Verification"
echo "========================================"
echo ""

# Track overall status
FAILED=0

# Function to print status
print_status() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓ $2${NC}"
    else
        echo -e "${RED}✗ $2${NC}"
        FAILED=1
    fi
}

# Check 1: Format check
echo "1. Checking code formatting..."
if cargo fmt --package $PACKAGE -- --check &> /tmp/fmt_check.log; then
    print_status 0 "Format check passed"
else
    print_status 1 "Format check failed"
    echo -e "${YELLOW}Run: cargo fmt --package $PACKAGE${NC}"
    cat /tmp/fmt_check.log
fi
echo ""

# Check 2: Clippy check (deny warnings)
echo "2. Running clippy (strict mode)..."
if cargo clippy --package $PACKAGE -- -D warnings &> /tmp/clippy_check.log; then
    print_status 0 "Clippy check passed (no warnings)"
else
    print_status 1 "Clippy check failed"
    echo -e "${YELLOW}Fix all clippy warnings${NC}"
    cat /tmp/clippy_check.log
fi
echo ""

# Check 3: Build check
echo "3. Building package..."
if cargo build --package $PACKAGE &> /tmp/build_check.log; then
    print_status 0 "Build successful"
else
    print_status 1 "Build failed"
    cat /tmp/build_check.log
fi
echo ""

# Check 4: Test check
echo "4. Running tests..."
if cargo test --package $PACKAGE &> /tmp/test_check.log; then
    print_status 0 "All tests passed"
else
    print_status 1 "Tests failed"
    cat /tmp/test_check.log
fi
echo ""

# Check 5: Documentation check
echo "5. Building documentation..."
if cargo doc --package $PACKAGE --no-deps &> /tmp/doc_check.log; then
    print_status 0 "Documentation built successfully"
else
    print_status 1 "Documentation build failed"
    cat /tmp/doc_check.log
fi
echo ""

# Check 6: Doc tests
echo "6. Running documentation tests..."
if cargo test --doc --package $PACKAGE &> /tmp/doctest_check.log; then
    print_status 0 "Documentation tests passed"
else
    print_status 1 "Documentation tests failed"
    cat /tmp/doctest_check.log
fi
echo ""

# Check 7: NIST/STIG comments check (basic grep)
echo "7. Checking for security compliance comments..."
SECURITY_FILES=("src/server.rs" "src/client.rs" "src/protocol.rs" "src/config.rs")
MISSING_NIST=0

for file in "${SECURITY_FILES[@]}"; do
    if [ -f "crates/$PACKAGE/$file" ]; then
        if grep -q "NIST 800-53" "crates/$PACKAGE/$file"; then
            echo -e "  ${GREEN}✓${NC} $file has NIST comments"
        else
            echo -e "  ${YELLOW}⚠${NC} $file missing NIST comments"
            MISSING_NIST=1
        fi
    fi
done

if [ $MISSING_NIST -eq 0 ]; then
    print_status 0 "NIST/STIG comments present"
else
    echo -e "${YELLOW}⚠ Warning: Some files missing NIST comments${NC}"
    echo -e "${YELLOW}  This is required for security-critical code${NC}"
fi
echo ""

# Summary
echo "========================================"
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ ALL CHECKS PASSED${NC}"
    echo "========================================"
    exit 0
else
    echo -e "${RED}✗ VERIFICATION FAILED${NC}"
    echo "========================================"
    echo ""
    echo "Please fix the issues above before committing."
    echo "See DEVELOPMENT_RULES.md for requirements."
    exit 1
fi
