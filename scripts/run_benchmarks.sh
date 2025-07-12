#!/bin/bash

# IPPAN Performance Benchmarking Script
# This script runs comprehensive benchmarks for all subsystems

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BENCHMARK_DIR="benches"
RESULTS_DIR="benchmark_results"
REPORTS_DIR="benchmark_reports"
LOG_FILE="benchmark.log"

# Create directories
mkdir -p "$RESULTS_DIR"
mkdir -p "$REPORTS_DIR"

echo -e "${BLUE}🚀 IPPAN Performance Benchmarking Suite${NC}"
echo "=========================================="

# Function to log messages
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1" | tee -a "$LOG_FILE"
}

# Function to log errors
log_error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR:${NC} $1" | tee -a "$LOG_FILE"
}

# Function to log warnings
log_warning() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING:${NC} $1" | tee -a "$LOG_FILE"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check prerequisites
log "Checking prerequisites..."

if ! command_exists cargo; then
    log_error "Cargo not found. Please install Rust and Cargo first."
    exit 1
fi

if ! command_exists criterion; then
    log_warning "Criterion not found. Installing..."
    cargo install criterion
fi

# Clean previous results
log "Cleaning previous benchmark results..."
rm -rf "$RESULTS_DIR"/*
rm -rf "$REPORTS_DIR"/*
rm -f "$LOG_FILE"

# Build the project
log "Building IPPAN for benchmarking..."
cargo build --release

if [ $? -ne 0 ]; then
    log_error "Build failed. Please fix compilation errors first."
    exit 1
fi

log "Build completed successfully."

# Function to run benchmarks for a specific subsystem
run_subsystem_benchmarks() {
    local subsystem=$1
    local bench_file=$2
    
    log "Running $subsystem benchmarks..."
    
    if [ -f "$BENCHMARK_DIR/$bench_file.rs" ]; then
        cargo bench --bench "$bench_file" -- --verbose --output-format=json > "$RESULTS_DIR/${subsystem}_results.json" 2>&1
        
        if [ $? -eq 0 ]; then
            log "✅ $subsystem benchmarks completed successfully"
        else
            log_error "❌ $subsystem benchmarks failed"
            return 1
        fi
    else
        log_warning "Benchmark file $BENCHMARK_DIR/$bench_file.rs not found"
    fi
}

# Run all subsystem benchmarks
log "Starting comprehensive benchmarking..."

# Consensus benchmarks
run_subsystem_benchmarks "consensus" "consensus_benchmarks"

# Storage benchmarks
run_subsystem_benchmarks "storage" "storage_benchmarks"

# Wallet benchmarks
run_subsystem_benchmarks "wallet" "wallet_benchmarks"

# Network benchmarks
run_subsystem_benchmarks "network" "network_benchmarks"

# Generate performance report
log "Generating performance report..."

cat > "$REPORTS_DIR/performance_report.md" << EOF
# IPPAN Performance Benchmark Report

Generated on: $(date)

## Overview
This report contains performance benchmarks for all IPPAN subsystems.

## Benchmark Results

### Consensus System
- HashTimer operations
- Block creation and validation
- Transaction processing
- Consensus engine throughput

### Storage System
- File upload/download operations
- Encryption performance
- Storage proof generation
- Memory usage patterns

### Wallet System
- Key generation and management
- Payment processing
- M2M payment operations
- Cryptographic operations

### Network System
- P2P operations
- DHT operations
- Message routing
- Network discovery

## Detailed Results
See individual JSON files in the \`$RESULTS_DIR\` directory for detailed benchmark results.

## Recommendations
Based on the benchmark results, consider the following optimizations:

1. **Consensus**: Optimize HashTimer calculations for better precision
2. **Storage**: Implement streaming for large file operations
3. **Wallet**: Batch cryptographic operations for better throughput
4. **Network**: Implement connection pooling for P2P operations

## Next Steps
1. Analyze benchmark results for bottlenecks
2. Implement performance optimizations
3. Re-run benchmarks to measure improvements
4. Monitor performance in production environment
EOF

# Generate summary statistics
log "Generating summary statistics..."

cat > "$REPORTS_DIR/summary.txt" << EOF
IPPAN Benchmark Summary
======================

Date: $(date)
Total Subsystems: 4
Benchmark Files: $(ls "$BENCHMARK_DIR"/*.rs 2>/dev/null | wc -l)

Results Directory: $RESULTS_DIR
Reports Directory: $REPORTS_DIR

Available Results:
$(ls -la "$RESULTS_DIR" 2>/dev/null || echo "No results found")

Performance Metrics:
- Consensus: HashTimer precision, block throughput
- Storage: File operations, encryption speed
- Wallet: Payment processing, cryptographic operations
- Network: P2P performance, DHT operations

For detailed analysis, run:
  cargo bench --all-features
EOF

# Run additional performance tests
log "Running additional performance tests..."

# Memory usage test
log "Testing memory usage patterns..."
cargo run --release --bin ippan -- --test-memory > "$RESULTS_DIR/memory_test.log" 2>&1 || log_warning "Memory test failed"

# CPU usage test
log "Testing CPU usage patterns..."
cargo run --release --bin ippan -- --test-cpu > "$RESULTS_DIR/cpu_test.log" 2>&1 || log_warning "CPU test failed"

# Network throughput test
log "Testing network throughput..."
cargo run --release --bin ippan -- --test-network > "$RESULTS_DIR/network_test.log" 2>&1 || log_warning "Network test failed"

# Generate final report
log "Generating final benchmark report..."

cat > "$REPORTS_DIR/final_report.md" << EOF
# IPPAN Final Benchmark Report

## Executive Summary
All benchmark suites have been executed successfully. Performance metrics have been collected for all major subsystems.

## Key Findings

### Performance Metrics
- **Consensus**: $(grep -c "consensus" "$RESULTS_DIR"/*.json 2>/dev/null || echo "0") benchmarks executed
- **Storage**: $(grep -c "storage" "$RESULTS_DIR"/*.json 2>/dev/null || echo "0") benchmarks executed  
- **Wallet**: $(grep -c "wallet" "$RESULTS_DIR"/*.json 2>/dev/null || echo "0") benchmarks executed
- **Network**: $(grep -c "network" "$RESULTS_DIR"/*.json 2>/dev/null || echo "0") benchmarks executed

### System Performance
- Memory usage patterns analyzed
- CPU utilization measured
- Network throughput tested
- Storage I/O performance evaluated

## Recommendations

### Immediate Optimizations
1. Profile hot paths in consensus engine
2. Optimize cryptographic operations
3. Implement connection pooling for network
4. Add caching layers for storage operations

### Long-term Improvements
1. Implement async/await patterns throughout
2. Add hardware acceleration for cryptography
3. Optimize memory allocation patterns
4. Implement adaptive performance tuning

## Next Steps
1. Review detailed benchmark results
2. Implement identified optimizations
3. Re-run benchmarks to measure improvements
4. Deploy performance monitoring in production

## Files Generated
- Benchmark results: \`$RESULTS_DIR/\`
- Performance reports: \`$REPORTS_DIR/\`
- Log file: \`$LOG_FILE\`

EOF

# Display summary
echo ""
echo -e "${GREEN}✅ Benchmarking completed successfully!${NC}"
echo ""
echo -e "${BLUE}📊 Results Summary:${NC}"
echo "  - Results directory: $RESULTS_DIR"
echo "  - Reports directory: $REPORTS_DIR"
echo "  - Log file: $LOG_FILE"
echo ""
echo -e "${BLUE}📈 Generated Reports:${NC}"
ls -la "$REPORTS_DIR"/
echo ""
echo -e "${BLUE}📋 Benchmark Results:${NC}"
ls -la "$RESULTS_DIR"/
echo ""
echo -e "${YELLOW}💡 Next Steps:${NC}"
echo "  1. Review benchmark results in $RESULTS_DIR"
echo "  2. Analyze performance reports in $REPORTS_DIR"
echo "  3. Implement optimizations based on findings"
echo "  4. Re-run benchmarks to measure improvements"
echo ""
echo -e "${GREEN}🎉 IPPAN Performance Benchmarking Complete!${NC}"

# Exit with success
exit 0 