#!/bin/bash

# IPPAN Test Runner Script
# Runs comprehensive tests for the IPPAN project

set -e

echo "🚀 IPPAN Test Suite"
echo "=================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    print_error "This script must be run from the IPPAN project root directory"
    exit 1
fi

# Build the project first
print_status "Building IPPAN project..."
cargo build --release

if [ $? -eq 0 ]; then
    print_success "Build completed successfully"
else
    print_error "Build failed"
    exit 1
fi

# Run cargo check
print_status "Running cargo check..."
cargo check

if [ $? -eq 0 ]; then
    print_success "Cargo check passed"
else
    print_warning "Cargo check had warnings or errors"
fi

# Run unit tests
print_status "Running unit tests..."
cargo test --lib --tests unit_tests --release

if [ $? -eq 0 ]; then
    print_success "Unit tests passed"
else
    print_error "Unit tests failed"
    exit 1
fi

# Run integration tests
print_status "Running integration tests..."
cargo test --lib --tests integration_tests --release

if [ $? -eq 0 ]; then
    print_success "Integration tests passed"
else
    print_error "Integration tests failed"
    exit 1
fi

# Run all tests
print_status "Running complete test suite..."
cargo test --release

if [ $? -eq 0 ]; then
    print_success "All tests passed"
else
    print_error "Some tests failed"
    exit 1
fi

# Run specific subsystem tests
print_status "Running consensus tests..."
cargo test consensus_tests --release

print_status "Running storage tests..."
cargo test storage_tests --release

print_status "Running network tests..."
cargo test network_tests --release

print_status "Running wallet tests..."
cargo test wallet_tests --release

print_status "Running DHT tests..."
cargo test dht_tests --release

print_status "Running staking tests..."
cargo test staking_tests --release

print_status "Running domain tests..."
cargo test domain_tests --release

print_status "Running API tests..."
cargo test api_tests --release

print_status "Running utility tests..."
cargo test utility_tests --release

# Performance tests (if available)
if [ -d "benches" ]; then
    print_status "Running benchmarks..."
    cargo bench
fi

# Code coverage (if grcov is available)
if command -v grcov &> /dev/null; then
    print_status "Generating code coverage report..."
    CARGO_INCREMENTAL=0 RUSTFLAGS='-Cinstrument-coverage' LLVM_PROFILE_FILE='cargo-test-%p-%m.profraw' cargo test
    grcov . --binary-path ./target/debug/ -s . -t html --branch --ignore-not-existing -o ./coverage/
    print_success "Coverage report generated in ./coverage/"
fi

# Clippy checks
print_status "Running clippy checks..."
cargo clippy -- -D warnings

if [ $? -eq 0 ]; then
    print_success "Clippy checks passed"
else
    print_warning "Clippy found some issues"
fi

# Security audit
print_status "Running security audit..."
cargo audit

if [ $? -eq 0 ]; then
    print_success "Security audit passed"
else
    print_warning "Security audit found some issues"
fi

echo ""
echo "🎉 Test suite completed!"
echo "========================"
echo ""
echo "Summary:"
echo "- ✅ Build completed"
echo "- ✅ Cargo check passed"
echo "- ✅ Unit tests passed"
echo "- ✅ Integration tests passed"
echo "- ✅ All tests passed"
echo "- ✅ Subsystem tests passed"
echo "- ✅ Clippy checks passed"
echo "- ✅ Security audit completed"
echo ""
echo "Next steps:"
echo "1. Review any warnings or issues"
echo "2. Run performance benchmarks"
echo "3. Check code coverage report"
echo "4. Address any security findings"
echo ""

print_success "All tests completed successfully!" 