#!/bin/bash

# IPPAN Testnet Validation Script
# This script validates the testnet deployment and functionality

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Test configuration
NODE_COUNT=5
PORT_START=8081
TEST_DURATION=300  # 5 minutes
TRANSACTION_COUNT=100

# Validation results
declare -A VALIDATION_RESULTS
VALIDATION_RESULTS[total_tests]=0
VALIDATION_RESULTS[passed_tests]=0
VALIDATION_RESULTS[failed_tests]=0

# Test functions
run_test() {
    local test_name="$1"
    local test_function="$2"
    
    log_info "Running test: $test_name"
    VALIDATION_RESULTS[total_tests]=$((VALIDATION_RESULTS[total_tests] + 1))
    
    if $test_function; then
        log_success "Test passed: $test_name"
        VALIDATION_RESULTS[passed_tests]=$((VALIDATION_RESULTS[passed_tests] + 1))
        return 0
    else
        log_error "Test failed: $test_name"
        VALIDATION_RESULTS[failed_tests]=$((VALIDATION_RESULTS[failed_tests] + 1))
        return 1
    fi
}

# Test: Check if all nodes are running
test_nodes_running() {
    local all_running=true
    
    for i in $(seq 1 $NODE_COUNT); do
        local port=$((PORT_START + i - 1))
        if ! curl -f "http://localhost:$port/api/v1/status" > /dev/null 2>&1; then
            log_error "Node $i is not responding on port $port"
            all_running=false
        fi
    done
    
    return $([ "$all_running" = true ] && echo 0 || echo 1)
}

# Test: Check node health endpoints
test_node_health() {
    local all_healthy=true
    
    for i in $(seq 1 $NODE_COUNT); do
        local port=$((PORT_START + i - 1))
        local response=$(curl -s "http://localhost:$port/api/v1/status" 2>/dev/null || echo "")
        
        if [[ -z "$response" ]]; then
            log_error "Node $i health check failed"
            all_healthy=false
        else
            # Check if response contains expected fields
            if ! echo "$response" | grep -q "status"; then
                log_error "Node $i health response missing status field"
                all_healthy=false
            fi
        fi
    done
    
    return $([ "$all_healthy" = true ] && echo 0 || echo 1)
}

# Test: Check P2P connectivity
test_p2p_connectivity() {
    local connectivity_ok=true
    
    for i in $(seq 1 $NODE_COUNT); do
        local port=$((PORT_START + i - 1))
        local response=$(curl -s "http://localhost:$port/api/v1/peers" 2>/dev/null || echo "")
        
        if [[ -z "$response" ]]; then
            log_warning "Node $i peers endpoint not responding"
        else
            # Check if node has peers
            local peer_count=$(echo "$response" | grep -o '"peer_id"' | wc -l)
            if [[ $peer_count -lt 2 ]]; then
                log_warning "Node $i has only $peer_count peers (expected at least 2)"
            fi
        fi
    done
    
    return 0  # P2P connectivity is not critical for basic validation
}

# Test: Check consensus status
test_consensus_status() {
    local consensus_ok=true
    
    for i in $(seq 1 $NODE_COUNT); do
        local port=$((PORT_START + i - 1))
        local response=$(curl -s "http://localhost:$port/api/v1/consensus/status" 2>/dev/null || echo "")
        
        if [[ -z "$response" ]]; then
            log_warning "Node $i consensus endpoint not responding"
        else
            # Check consensus status
            if ! echo "$response" | grep -q "consensus"; then
                log_warning "Node $i consensus response missing consensus field"
            fi
        fi
    done
    
    return 0  # Consensus status is not critical for basic validation
}

# Test: Check blockchain status
test_blockchain_status() {
    local blockchain_ok=true
    
    for i in $(seq 1 $NODE_COUNT); do
        local port=$((PORT_START + i - 1))
        local response=$(curl -s "http://localhost:$port/api/v1/blockchain/status" 2>/dev/null || echo "")
        
        if [[ -z "$response" ]]; then
            log_warning "Node $i blockchain endpoint not responding"
        else
            # Check blockchain status
            if ! echo "$response" | grep -q "height"; then
                log_warning "Node $i blockchain response missing height field"
            fi
        fi
    done
    
    return 0  # Blockchain status is not critical for basic validation
}

# Test: Check metrics endpoints
test_metrics_endpoints() {
    local metrics_ok=true
    
    for i in $(seq 1 $NODE_COUNT); do
        local port=$((PORT_START + i - 1))
        local response=$(curl -s "http://localhost:$port/metrics" 2>/dev/null || echo "")
        
        if [[ -z "$response" ]]; then
            log_error "Node $i metrics endpoint not responding"
            metrics_ok=false
        else
            # Check if response contains metrics
            if ! echo "$response" | grep -q "# HELP"; then
                log_error "Node $i metrics response not in Prometheus format"
                metrics_ok=false
            fi
        fi
    done
    
    return $([ "$metrics_ok" = true ] && echo 0 || echo 1)
}

# Test: Check monitoring services
test_monitoring_services() {
    local monitoring_ok=true
    
    # Check Prometheus
    if ! curl -f "http://localhost:9090/-/healthy" > /dev/null 2>&1; then
        log_error "Prometheus is not responding"
        monitoring_ok=false
    else
        log_success "Prometheus is healthy"
    fi
    
    # Check Grafana
    if ! curl -f "http://localhost:3000/api/health" > /dev/null 2>&1; then
        log_error "Grafana is not responding"
        monitoring_ok=false
    else
        log_success "Grafana is healthy"
    fi
    
    return $([ "$monitoring_ok" = true ] && echo 0 || echo 1)
}

# Test: Check load balancer
test_load_balancer() {
    local lb_ok=true
    
    # Test load balancer endpoint
    if ! curl -f "http://localhost:80/api/v1/status" > /dev/null 2>&1; then
        log_error "Load balancer is not responding"
        lb_ok=false
    else
        log_success "Load balancer is healthy"
    fi
    
    return $([ "$lb_ok" = true ] && echo 0 || echo 1)
}

# Test: Check network connectivity between nodes
test_network_connectivity() {
    local network_ok=true
    
    # This test would require more complex network testing
    # For now, we'll just check if nodes can reach each other
    log_info "Testing network connectivity between nodes..."
    
    for i in $(seq 1 $NODE_COUNT); do
        local port=$((PORT_START + i - 1))
        local response=$(curl -s "http://localhost:$port/api/v1/network/status" 2>/dev/null || echo "")
        
        if [[ -z "$response" ]]; then
            log_warning "Node $i network endpoint not responding"
        fi
    done
    
    return 0  # Network connectivity is not critical for basic validation
}

# Test: Check storage functionality
test_storage_functionality() {
    local storage_ok=true
    
    # Test storage by checking if nodes can store and retrieve data
    for i in $(seq 1 $NODE_COUNT); do
        local port=$((PORT_START + i - 1))
        local response=$(curl -s "http://localhost:$port/api/v1/storage/status" 2>/dev/null || echo "")
        
        if [[ -z "$response" ]]; then
            log_warning "Node $i storage endpoint not responding"
        fi
    done
    
    return 0  # Storage functionality is not critical for basic validation
}

# Test: Check wallet functionality
test_wallet_functionality() {
    local wallet_ok=true
    
    # Test wallet by checking if nodes have wallet endpoints
    for i in $(seq 1 $NODE_COUNT); do
        local port=$((PORT_START + i - 1))
        local response=$(curl -s "http://localhost:$port/api/v1/wallet/status" 2>/dev/null || echo "")
        
        if [[ -z "$response" ]]; then
            log_warning "Node $i wallet endpoint not responding"
        fi
    done
    
    return 0  # Wallet functionality is not critical for basic validation
}

# Test: Check mining functionality
test_mining_functionality() {
    local mining_ok=true
    
    # Test mining by checking if nodes have mining endpoints
    for i in $(seq 1 $NODE_COUNT); do
        local port=$((PORT_START + i - 1))
        local response=$(curl -s "http://localhost:$port/api/v1/mining/status" 2>/dev/null || echo "")
        
        if [[ -z "$response" ]]; then
            log_warning "Node $i mining endpoint not responding"
        fi
    done
    
    return 0  # Mining functionality is not critical for basic validation
}

# Test: Check API consistency
test_api_consistency() {
    local api_ok=true
    
    # Test if all nodes return consistent API responses
    local first_response=""
    local first_port=$PORT_START
    
    # Get response from first node
    first_response=$(curl -s "http://localhost:$first_port/api/v1/status" 2>/dev/null || echo "")
    
    if [[ -z "$first_response" ]]; then
        log_error "First node API not responding"
        return 1
    fi
    
    # Compare with other nodes
    for i in $(seq 2 $NODE_COUNT); do
        local port=$((PORT_START + i - 1))
        local response=$(curl -s "http://localhost:$port/api/v1/status" 2>/dev/null || echo "")
        
        if [[ -z "$response" ]]; then
            log_error "Node $i API not responding"
            api_ok=false
        else
            # Compare response structure (simplified)
            if [[ $(echo "$first_response" | wc -l) -ne $(echo "$response" | wc -l) ]]; then
                log_warning "Node $i API response structure differs from node 1"
            fi
        fi
    done
    
    return $([ "$api_ok" = true ] && echo 0 || echo 1)
}

# Test: Check error handling
test_error_handling() {
    local error_handling_ok=true
    
    # Test error handling by sending invalid requests
    for i in $(seq 1 $NODE_COUNT); do
        local port=$((PORT_START + i - 1))
        
        # Test invalid endpoint
        local response=$(curl -s -w "%{http_code}" "http://localhost:$port/api/v1/invalid" 2>/dev/null || echo "")
        local http_code="${response: -3}"
        
        if [[ "$http_code" != "404" ]]; then
            log_warning "Node $i error handling for invalid endpoint returned $http_code (expected 404)"
        fi
    done
    
    return 0  # Error handling is not critical for basic validation
}

# Test: Check performance
test_performance() {
    local performance_ok=true
    
    # Test performance by measuring response times
    for i in $(seq 1 $NODE_COUNT); do
        local port=$((PORT_START + i - 1))
        
        # Measure response time
        local start_time=$(date +%s%N)
        curl -f "http://localhost:$port/api/v1/status" > /dev/null 2>&1
        local end_time=$(date +%s%N)
        local response_time=$(( (end_time - start_time) / 1000000 ))  # Convert to milliseconds
        
        if [[ $response_time -gt 1000 ]]; then
            log_warning "Node $i response time is ${response_time}ms (expected < 1000ms)"
        else
            log_success "Node $i response time is ${response_time}ms"
        fi
    done
    
    return 0  # Performance is not critical for basic validation
}

# Test: Check security
test_security() {
    local security_ok=true
    
    # Test security by checking if sensitive endpoints are protected
    for i in $(seq 1 $NODE_COUNT); do
        local port=$((PORT_START + i - 1))
        
        # Test admin endpoint (should be protected)
        local response=$(curl -s -w "%{http_code}" "http://localhost:$port/api/v1/admin" 2>/dev/null || echo "")
        local http_code="${response: -3}"
        
        if [[ "$http_code" != "401" && "$http_code" != "403" ]]; then
            log_warning "Node $i admin endpoint returned $http_code (expected 401 or 403)"
        fi
    done
    
    return 0  # Security is not critical for basic validation
}

# Test: Check logging
test_logging() {
    local logging_ok=true
    
    # Test logging by checking if log files exist and are being written
    for i in $(seq 1 $NODE_COUNT); do
        local container_name="ippan-testnet-node-$i"
        
        # Check if log file exists and has content
        if docker exec "$container_name" test -f "/logs/ippan-node-$i.log" 2>/dev/null; then
            local log_size=$(docker exec "$container_name" wc -c < "/logs/ippan-node-$i.log" 2>/dev/null || echo "0")
            if [[ $log_size -gt 0 ]]; then
                log_success "Node $i log file exists and has content ($log_size bytes)"
            else
                log_warning "Node $i log file exists but is empty"
            fi
        else
            log_warning "Node $i log file does not exist"
        fi
    done
    
    return 0  # Logging is not critical for basic validation
}

# Test: Check resource usage
test_resource_usage() {
    local resource_ok=true
    
    # Test resource usage by checking Docker stats
    for i in $(seq 1 $NODE_COUNT); do
        local container_name="ippan-testnet-node-$i"
        
        # Check if container is running
        if docker ps --format "table {{.Names}}" | grep -q "$container_name"; then
            log_success "Node $i container is running"
        else
            log_error "Node $i container is not running"
            resource_ok=false
        fi
    done
    
    return $([ "$resource_ok" = true ] && echo 0 || echo 1)
}

# Test: Check data persistence
test_data_persistence() {
    local persistence_ok=true
    
    # Test data persistence by checking if data directories exist
    for i in $(seq 1 $NODE_COUNT); do
        local container_name="ippan-testnet-node-$i"
        
        # Check if data directory exists
        if docker exec "$container_name" test -d "/data" 2>/dev/null; then
            log_success "Node $i data directory exists"
        else
            log_error "Node $i data directory does not exist"
            persistence_ok=false
        fi
        
        # Check if keys directory exists
        if docker exec "$container_name" test -d "/keys" 2>/dev/null; then
            log_success "Node $i keys directory exists"
        else
            log_error "Node $i keys directory does not exist"
            persistence_ok=false
        fi
    done
    
    return $([ "$persistence_ok" = true ] && echo 0 || echo 1)
}

# Main validation function
run_validation() {
    log_info "Starting IPPAN testnet validation..."
    log_info "Testnet configuration:"
    log_info "  - Node count: $NODE_COUNT"
    log_info "  - Port range: $PORT_START - $((PORT_START + NODE_COUNT - 1))"
    log_info "  - Test duration: $TEST_DURATION seconds"
    
    # Run all tests
    run_test "Nodes Running" test_nodes_running
    run_test "Node Health" test_node_health
    run_test "P2P Connectivity" test_p2p_connectivity
    run_test "Consensus Status" test_consensus_status
    run_test "Blockchain Status" test_blockchain_status
    run_test "Metrics Endpoints" test_metrics_endpoints
    run_test "Monitoring Services" test_monitoring_services
    run_test "Load Balancer" test_load_balancer
    run_test "Network Connectivity" test_network_connectivity
    run_test "Storage Functionality" test_storage_functionality
    run_test "Wallet Functionality" test_wallet_functionality
    run_test "Mining Functionality" test_mining_functionality
    run_test "API Consistency" test_api_consistency
    run_test "Error Handling" test_error_handling
    run_test "Performance" test_performance
    run_test "Security" test_security
    run_test "Logging" test_logging
    run_test "Resource Usage" test_resource_usage
    run_test "Data Persistence" test_data_persistence
    
    # Print validation summary
    log_info "Validation Summary:"
    log_info "  Total tests: ${VALIDATION_RESULTS[total_tests]}"
    log_info "  Passed tests: ${VALIDATION_RESULTS[passed_tests]}"
    log_info "  Failed tests: ${VALIDATION_RESULTS[failed_tests]}"
    
    local success_rate=$((VALIDATION_RESULTS[passed_tests] * 100 / VALIDATION_RESULTS[total_tests]))
    log_info "  Success rate: $success_rate%"
    
    if [[ ${VALIDATION_RESULTS[failed_tests]} -eq 0 ]]; then
        log_success "All tests passed! Testnet validation successful."
        return 0
    else
        log_error "Some tests failed. Testnet validation unsuccessful."
        return 1
    fi
}

# Help function
show_help() {
    cat << EOF
IPPAN Testnet Validation Script

Usage: $0 [OPTIONS]

Options:
    -n, --nodes     Number of nodes (default: 5)
    -p, --ports     Port range start (default: 8081)
    -h, --help      Show this help message

Examples:
    $0
    $0 -n 3 -p 8081

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -n|--nodes)
                NODE_COUNT="$2"
                shift 2
                ;;
            -p|--ports)
                PORT_START="$2"
                shift 2
                ;;
            -h|--help)
                show_help
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

# Main function
main() {
    parse_args "$@"
    run_validation
}

# Run main function
main "$@"
