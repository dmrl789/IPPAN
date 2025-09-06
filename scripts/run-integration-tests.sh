#!/bin/bash

# IPPAN Integration Test Runner
# Comprehensive end-to-end testing of the complete IPPAN system

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
TEST_TIMEOUT=300  # 5 minutes per test suite
LOG_DIR="logs/integration-tests"
REPORT_DIR="reports/integration-tests"

# Create directories
mkdir -p "$LOG_DIR"
mkdir -p "$REPORT_DIR"

# Function to print colored output
print_status() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] ✅ $1${NC}"
}

print_error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ❌ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] ⚠️  $1${NC}"
}

# Function to run a test suite with timeout
run_test_suite() {
    local test_name="$1"
    local test_command="$2"
    local log_file="$LOG_DIR/${test_name}.log"
    local report_file="$REPORT_DIR/${test_name}.json"
    
    print_status "Starting $test_name..."
    
    # Run test with timeout
    if timeout "$TEST_TIMEOUT" $test_command > "$log_file" 2>&1; then
        print_success "$test_name completed successfully"
        echo "{\"test\": \"$test_name\", \"status\": \"passed\", \"timestamp\": \"$(date -Iseconds)\"}" > "$report_file"
        return 0
    else
        local exit_code=$?
        print_error "$test_name failed (exit code: $exit_code)"
        echo "{\"test\": \"$test_name\", \"status\": \"failed\", \"exit_code\": $exit_code, \"timestamp\": \"$(date -Iseconds)\"}" > "$report_file"
        return $exit_code
    fi
}

# Function to check prerequisites
check_prerequisites() {
    print_status "Checking prerequisites..."
    
    # Check if Rust is installed
    if ! command -v cargo &> /dev/null; then
        print_error "Rust/Cargo is not installed"
        exit 1
    fi
    
    # Check if the project builds
    print_status "Building project..."
    if ! cargo build --release; then
        print_error "Project build failed"
        exit 1
    fi
    
    print_success "Prerequisites check passed"
}

# Function to run all integration tests
run_all_tests() {
    local start_time=$(date +%s)
    local failed_tests=0
    local total_tests=0
    
    print_status "🚀 Starting IPPAN Integration Test Suite"
    print_status "Test timeout: ${TEST_TIMEOUT}s per suite"
    print_status "Log directory: $LOG_DIR"
    print_status "Report directory: $REPORT_DIR"
    
    # Test suites
    local test_suites=(
        "unit_tests:cargo test --lib --release"
        "integration_tests:cargo test --test integration --release"
        "performance_tests:cargo test --test performance_integration --release"
        "security_tests:cargo test --test security_integration --release"
        "end_to_end_tests:cargo test --test end_to_end_integration --release"
        "load_tests:cargo test --test load_tests --release"
        "stress_tests:cargo test --test stress_tests --release"
    )
    
    # Run each test suite
    for test_suite in "${test_suites[@]}"; do
        IFS=':' read -r test_name test_command <<< "$test_suite"
        total_tests=$((total_tests + 1))
        
        if ! run_test_suite "$test_name" "$test_command"; then
            failed_tests=$((failed_tests + 1))
        fi
    done
    
    # Calculate results
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    local passed_tests=$((total_tests - failed_tests))
    
    # Print summary
    echo
    print_status "📊 Integration Test Results Summary"
    print_status "Total tests: $total_tests"
    print_status "Passed: $passed_tests"
    print_status "Failed: $failed_tests"
    print_status "Duration: ${duration}s"
    
    if [ $failed_tests -eq 0 ]; then
        print_success "🎉 All integration tests passed!"
        return 0
    else
        print_error "💥 $failed_tests test suite(s) failed"
        return 1
    fi
}

# Function to run specific test suite
run_specific_test() {
    local test_name="$1"
    
    case "$test_name" in
        "unit")
            run_test_suite "unit_tests" "cargo test --lib --release"
            ;;
        "integration")
            run_test_suite "integration_tests" "cargo test --test integration --release"
            ;;
        "performance")
            run_test_suite "performance_tests" "cargo test --test performance_integration --release"
            ;;
        "security")
            run_test_suite "security_tests" "cargo test --test security_integration --release"
            ;;
        "e2e")
            run_test_suite "end_to_end_tests" "cargo test --test end_to_end_integration --release"
            ;;
        "load")
            run_test_suite "load_tests" "cargo test --test load_tests --release"
            ;;
        "stress")
            run_test_suite "stress_tests" "cargo test --test stress_tests --release"
            ;;
        *)
            print_error "Unknown test suite: $test_name"
            print_status "Available test suites: unit, integration, performance, security, e2e, load, stress"
            exit 1
            ;;
    esac
}

# Function to generate test report
generate_report() {
    print_status "Generating test report..."
    
    local report_file="$REPORT_DIR/integration_test_report.html"
    
    cat > "$report_file" << EOF
<!DOCTYPE html>
<html>
<head>
    <title>IPPAN Integration Test Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .header { background-color: #f0f0f0; padding: 20px; border-radius: 5px; }
        .test-result { margin: 10px 0; padding: 10px; border-radius: 3px; }
        .passed { background-color: #d4edda; border: 1px solid #c3e6cb; }
        .failed { background-color: #f8d7da; border: 1px solid #f5c6cb; }
        .summary { background-color: #e2e3e5; padding: 15px; border-radius: 5px; margin: 20px 0; }
    </style>
</head>
<body>
    <div class="header">
        <h1>IPPAN Integration Test Report</h1>
        <p>Generated: $(date)</p>
    </div>
    
    <div class="summary">
        <h2>Test Summary</h2>
        <p>Total Tests: $(ls -1 "$REPORT_DIR"/*.json | wc -l)</p>
        <p>Passed: $(grep -c '"status": "passed"' "$REPORT_DIR"/*.json)</p>
        <p>Failed: $(grep -c '"status": "failed"' "$REPORT_DIR"/*.json)</p>
    </div>
    
    <h2>Test Results</h2>
EOF

    # Add test results
    for json_file in "$REPORT_DIR"/*.json; do
        if [ -f "$json_file" ]; then
            local test_name=$(basename "$json_file" .json)
            local status=$(grep -o '"status": "[^"]*"' "$json_file" | cut -d'"' -f4)
            local timestamp=$(grep -o '"timestamp": "[^"]*"' "$json_file" | cut -d'"' -f4)
            
            if [ "$status" = "passed" ]; then
                echo "    <div class=\"test-result passed\">" >> "$report_file"
                echo "        <h3>✅ $test_name</h3>" >> "$report_file"
            else
                echo "    <div class=\"test-result failed\">" >> "$report_file"
                echo "        <h3>❌ $test_name</h3>" >> "$report_file"
            fi
            
            echo "        <p>Status: $status</p>" >> "$report_file"
            echo "        <p>Timestamp: $timestamp</p>" >> "$report_file"
            echo "    </div>" >> "$report_file"
        fi
    done
    
    cat >> "$report_file" << EOF
</body>
</html>
EOF
    
    print_success "Test report generated: $report_file"
}

# Function to show help
show_help() {
    echo "IPPAN Integration Test Runner"
    echo
    echo "Usage: $0 [OPTIONS] [TEST_SUITE]"
    echo
    echo "Options:"
    echo "  -h, --help     Show this help message"
    echo "  -r, --report   Generate HTML test report"
    echo "  -t, --timeout  Set test timeout in seconds (default: 300)"
    echo
    echo "Test Suites:"
    echo "  unit          Run unit tests only"
    echo "  integration   Run integration tests only"
    echo "  performance   Run performance tests only"
    echo "  security      Run security tests only"
    echo "  e2e           Run end-to-end tests only"
    echo "  load          Run load tests only"
    echo "  stress        Run stress tests only"
    echo "  all           Run all test suites (default)"
    echo
    echo "Examples:"
    echo "  $0                    # Run all tests"
    echo "  $0 unit               # Run unit tests only"
    echo "  $0 --report           # Run all tests and generate report"
    echo "  $0 -t 600 performance # Run performance tests with 10min timeout"
}

# Main function
main() {
    local test_suite="all"
    local generate_report_flag=false
    local timeout_set=false
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            -r|--report)
                generate_report_flag=true
                shift
                ;;
            -t|--timeout)
                TEST_TIMEOUT="$2"
                timeout_set=true
                shift 2
                ;;
            unit|integration|performance|security|e2e|load|stress|all)
                test_suite="$1"
                shift
                ;;
            *)
                print_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
    
    # Check prerequisites
    check_prerequisites
    
    # Run tests
    if [ "$test_suite" = "all" ]; then
        if run_all_tests; then
            if [ "$generate_report_flag" = true ]; then
                generate_report
            fi
            exit 0
        else
            if [ "$generate_report_flag" = true ]; then
                generate_report
            fi
            exit 1
        fi
    else
        if run_specific_test "$test_suite"; then
            if [ "$generate_report_flag" = true ]; then
                generate_report
            fi
            exit 0
        else
            if [ "$generate_report_flag" = true ]; then
                generate_report
            fi
            exit 1
        fi
    fi
}

# Run main function
main "$@"
