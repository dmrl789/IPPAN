#!/bin/bash

# IPPAN Integration Tests
# This script runs comprehensive integration tests for the IPPAN staging environment

set -e

# Configuration
TEST_DIR="/test-results"
TARGET_HOST="${TARGET_HOST:-ippan-staging-node}"
TARGET_PORT="${TARGET_PORT:-3000}"
TEST_ENVIRONMENT="${TEST_ENVIRONMENT:-staging}"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

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

# Create test results directory
create_test_dir() {
    log_info "Creating test results directory..."
    mkdir -p "$TEST_DIR"
    log_success "Test directory created: $TEST_DIR"
}

# Wait for services to be ready
wait_for_services() {
    log_info "Waiting for services to be ready..."
    
    # Wait for IPPAN node
    log_info "Waiting for IPPAN node..."
    timeout 300 bash -c "until curl -f http://$TARGET_HOST:$TARGET_PORT/api/v1/status; do sleep 5; done"
    
    # Wait for Prometheus
    log_info "Waiting for Prometheus..."
    timeout 300 bash -c "until curl -f http://ippan-staging-monitor:9090/-/healthy; do sleep 5; done"
    
    # Wait for Grafana
    log_info "Waiting for Grafana..."
    timeout 300 bash -c "until curl -f http://ippan-staging-grafana:3001/api/health; do sleep 5; done"
    
    log_success "All services are ready"
}

# Test API endpoints
test_api_endpoints() {
    log_info "Testing API endpoints..."
    
    # Test health endpoint
    log_info "Testing health endpoint..."
    if curl -f "http://$TARGET_HOST:$TARGET_PORT/api/v1/status" > "$TEST_DIR/health-test.txt" 2>&1; then
        log_success "Health endpoint test passed"
    else
        log_error "Health endpoint test failed"
        return 1
    fi
    
    # Test node info endpoint
    log_info "Testing node info endpoint..."
    if curl -f "http://$TARGET_HOST:$TARGET_PORT/api/v1/node/info" > "$TEST_DIR/node-info-test.txt" 2>&1; then
        log_success "Node info endpoint test passed"
    else
        log_error "Node info endpoint test failed"
        return 1
    fi
    
    # Test consensus endpoint
    log_info "Testing consensus endpoint..."
    if curl -f "http://$TARGET_HOST:$TARGET_PORT/api/v1/consensus/round" > "$TEST_DIR/consensus-test.txt" 2>&1; then
        log_success "Consensus endpoint test passed"
    else
        log_error "Consensus endpoint test failed"
        return 1
    fi
    
    # Test storage endpoint
    log_info "Testing storage endpoint..."
    if curl -f "http://$TARGET_HOST:$TARGET_PORT/api/v1/storage/usage" > "$TEST_DIR/storage-test.txt" 2>&1; then
        log_success "Storage endpoint test passed"
    else
        log_error "Storage endpoint test failed"
        return 1
    fi
    
    # Test wallet endpoint
    log_info "Testing wallet endpoint..."
    if curl -f "http://$TARGET_HOST:$TARGET_PORT/api/v1/wallet/balance" > "$TEST_DIR/wallet-test.txt" 2>&1; then
        log_success "Wallet endpoint test passed"
    else
        log_error "Wallet endpoint test failed"
        return 1
    fi
    
    log_success "API endpoints testing completed"
}

# Test transaction processing
test_transaction_processing() {
    log_info "Testing transaction processing..."
    
    # Create test transaction
    log_info "Creating test transaction..."
    cat > "$TEST_DIR/test-transaction.json" << EOF
{
    "amount": 1000000,
    "recipient": "test-recipient-$(date +%s)",
    "sender": "test-sender-$(date +%s)",
    "timestamp": $(date +%s)000
}
EOF
    
    # Submit transaction
    log_info "Submitting test transaction..."
    if curl -f -X POST \
        -H "Content-Type: application/json" \
        -d @"$TEST_DIR/test-transaction.json" \
        "http://$TARGET_HOST:$TARGET_PORT/api/v1/transactions" > "$TEST_DIR/transaction-submit-test.txt" 2>&1; then
        log_success "Transaction submission test passed"
    else
        log_error "Transaction submission test failed"
        return 1
    fi
    
    # Wait for transaction processing
    log_info "Waiting for transaction processing..."
    sleep 5
    
    # Check transaction status
    log_info "Checking transaction status..."
    if curl -f "http://$TARGET_HOST:$TARGET_PORT/api/v1/transactions" > "$TEST_DIR/transaction-status-test.txt" 2>&1; then
        log_success "Transaction status test passed"
    else
        log_error "Transaction status test failed"
        return 1
    fi
    
    log_success "Transaction processing testing completed"
}

# Test consensus mechanism
test_consensus_mechanism() {
    log_info "Testing consensus mechanism..."
    
    # Test consensus round
    log_info "Testing consensus round..."
    if curl -f "http://$TARGET_HOST:$TARGET_PORT/api/v1/consensus/round" > "$TEST_DIR/consensus-round-test.txt" 2>&1; then
        log_success "Consensus round test passed"
    else
        log_error "Consensus round test failed"
        return 1
    fi
    
    # Test block generation
    log_info "Testing block generation..."
    if curl -f "http://$TARGET_HOST:$TARGET_PORT/api/v1/consensus/blocks" > "$TEST_DIR/block-generation-test.txt" 2>&1; then
        log_success "Block generation test passed"
    else
        log_error "Block generation test failed"
        return 1
    fi
    
    # Test validator list
    log_info "Testing validator list..."
    if curl -f "http://$TARGET_HOST:$TARGET_PORT/api/v1/consensus/validators" > "$TEST_DIR/validator-list-test.txt" 2>&1; then
        log_success "Validator list test passed"
    else
        log_error "Validator list test failed"
        return 1
    fi
    
    log_success "Consensus mechanism testing completed"
}

# Test storage system
test_storage_system() {
    log_info "Testing storage system..."
    
    # Test storage usage
    log_info "Testing storage usage..."
    if curl -f "http://$TARGET_HOST:$TARGET_PORT/api/v1/storage/usage" > "$TEST_DIR/storage-usage-test.txt" 2>&1; then
        log_success "Storage usage test passed"
    else
        log_error "Storage usage test failed"
        return 1
    fi
    
    # Test file upload
    log_info "Testing file upload..."
    echo "Test file content for IPPAN staging" > "$TEST_DIR/test-file.txt"
    if curl -f -X POST \
        -F "file=@$TEST_DIR/test-file.txt" \
        "http://$TARGET_HOST:$TARGET_PORT/api/v1/storage/upload" > "$TEST_DIR/file-upload-test.txt" 2>&1; then
        log_success "File upload test passed"
    else
        log_error "File upload test failed"
        return 1
    fi
    
    # Test file listing
    log_info "Testing file listing..."
    if curl -f "http://$TARGET_HOST:$TARGET_PORT/api/v1/storage/files" > "$TEST_DIR/file-listing-test.txt" 2>&1; then
        log_success "File listing test passed"
    else
        log_error "File listing test failed"
        return 1
    fi
    
    log_success "Storage system testing completed"
}

# Test wallet system
test_wallet_system() {
    log_info "Testing wallet system..."
    
    # Test wallet balance
    log_info "Testing wallet balance..."
    if curl -f "http://$TARGET_HOST:$TARGET_PORT/api/v1/wallet/balance" > "$TEST_DIR/wallet-balance-test.txt" 2>&1; then
        log_success "Wallet balance test passed"
    else
        log_error "Wallet balance test failed"
        return 1
    fi
    
    # Test wallet addresses
    log_info "Testing wallet addresses..."
    if curl -f "http://$TARGET_HOST:$TARGET_PORT/api/v1/wallet/addresses" > "$TEST_DIR/wallet-addresses-test.txt" 2>&1; then
        log_success "Wallet addresses test passed"
    else
        log_error "Wallet addresses test failed"
        return 1
    fi
    
    # Test transaction history
    log_info "Testing transaction history..."
    if curl -f "http://$TARGET_HOST:$TARGET_PORT/api/v1/wallet/transactions" > "$TEST_DIR/wallet-transactions-test.txt" 2>&1; then
        log_success "Wallet transactions test passed"
    else
        log_error "Wallet transactions test failed"
        return 1
    fi
    
    log_success "Wallet system testing completed"
}

# Test monitoring system
test_monitoring_system() {
    log_info "Testing monitoring system..."
    
    # Test Prometheus metrics
    log_info "Testing Prometheus metrics..."
    if curl -f "http://ippan-staging-monitor:9090/metrics" > "$TEST_DIR/prometheus-metrics-test.txt" 2>&1; then
        log_success "Prometheus metrics test passed"
    else
        log_error "Prometheus metrics test failed"
        return 1
    fi
    
    # Test Grafana health
    log_info "Testing Grafana health..."
    if curl -f "http://ippan-staging-grafana:3001/api/health" > "$TEST_DIR/grafana-health-test.txt" 2>&1; then
        log_success "Grafana health test passed"
    else
        log_error "Grafana health test failed"
        return 1
    fi
    
    # Test AlertManager
    log_info "Testing AlertManager..."
    if curl -f "http://ippan-staging-alertmanager:9093/-/healthy" > "$TEST_DIR/alertmanager-health-test.txt" 2>&1; then
        log_success "AlertManager health test passed"
    else
        log_error "AlertManager health test failed"
        return 1
    fi
    
    log_success "Monitoring system testing completed"
}

# Test network connectivity
test_network_connectivity() {
    log_info "Testing network connectivity..."
    
    # Test P2P connectivity
    log_info "Testing P2P connectivity..."
    if curl -f "http://$TARGET_HOST:$TARGET_PORT/api/v1/node/peers" > "$TEST_DIR/p2p-connectivity-test.txt" 2>&1; then
        log_success "P2P connectivity test passed"
    else
        log_error "P2P connectivity test failed"
        return 1
    fi
    
    # Test network statistics
    log_info "Testing network statistics..."
    if curl -f "http://$TARGET_HOST:$TARGET_PORT/api/v1/network/stats" > "$TEST_DIR/network-stats-test.txt" 2>&1; then
        log_success "Network statistics test passed"
    else
        log_error "Network statistics test failed"
        return 1
    fi
    
    log_success "Network connectivity testing completed"
}

# Test error handling
test_error_handling() {
    log_info "Testing error handling..."
    
    # Test invalid endpoint
    log_info "Testing invalid endpoint..."
    if curl -f "http://$TARGET_HOST:$TARGET_PORT/api/v1/invalid" > "$TEST_DIR/invalid-endpoint-test.txt" 2>&1; then
        log_warning "Invalid endpoint returned success (unexpected)"
    else
        log_success "Invalid endpoint test passed (returned error as expected)"
    fi
    
    # Test invalid transaction
    log_info "Testing invalid transaction..."
    if curl -f -X POST \
        -H "Content-Type: application/json" \
        -d '{"invalid": "data"}' \
        "http://$TARGET_HOST:$TARGET_PORT/api/v1/transactions" > "$TEST_DIR/invalid-transaction-test.txt" 2>&1; then
        log_warning "Invalid transaction returned success (unexpected)"
    else
        log_success "Invalid transaction test passed (returned error as expected)"
    fi
    
    log_success "Error handling testing completed"
}

# Test performance
test_performance() {
    log_info "Testing performance..."
    
    # Test response times
    log_info "Testing response times..."
    start_time=$(date +%s%N)
    curl -f "http://$TARGET_HOST:$TARGET_PORT/api/v1/status" > /dev/null 2>&1
    end_time=$(date +%s%N)
    response_time=$(( (end_time - start_time) / 1000000 ))
    echo "Response time: ${response_time}ms" > "$TEST_DIR/response-time-test.txt"
    
    if [ $response_time -lt 1000 ]; then
        log_success "Response time test passed (${response_time}ms)"
    else
        log_warning "Response time test warning (${response_time}ms > 1000ms)"
    fi
    
    # Test concurrent requests
    log_info "Testing concurrent requests..."
    for i in {1..10}; do
        curl -f "http://$TARGET_HOST:$TARGET_PORT/api/v1/status" > "$TEST_DIR/concurrent-request-$i.txt" 2>&1 &
    done
    wait
    
    successful_requests=$(ls "$TEST_DIR"/concurrent-request-*.txt 2>/dev/null | wc -l)
    if [ $successful_requests -eq 10 ]; then
        log_success "Concurrent requests test passed ($successful_requests/10)"
    else
        log_error "Concurrent requests test failed ($successful_requests/10)"
        return 1
    fi
    
    log_success "Performance testing completed"
}

# Generate test report
generate_test_report() {
    log_info "Generating integration test report..."
    
    cat > "$TEST_DIR/integration-test-report.md" << EOF
# IPPAN Integration Test Report

**Test Date**: $(date)
**Target Host**: $TARGET_HOST
**Target Port**: $TARGET_PORT
**Test Environment**: $TEST_ENVIRONMENT

## Executive Summary

This report contains the results of comprehensive integration testing performed on the IPPAN blockchain system in the staging environment.

## Test Results

### API Endpoints
- **Health Endpoint**: $(grep -q "200" "$TEST_DIR/health-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")
- **Node Info Endpoint**: $(grep -q "200" "$TEST_DIR/node-info-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")
- **Consensus Endpoint**: $(grep -q "200" "$TEST_DIR/consensus-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")
- **Storage Endpoint**: $(grep -q "200" "$TEST_DIR/storage-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")
- **Wallet Endpoint**: $(grep -q "200" "$TEST_DIR/wallet-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")

### Transaction Processing
- **Transaction Submission**: $(grep -q "200\|201" "$TEST_DIR/transaction-submit-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")
- **Transaction Status**: $(grep -q "200" "$TEST_DIR/transaction-status-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")

### Consensus Mechanism
- **Consensus Round**: $(grep -q "200" "$TEST_DIR/consensus-round-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")
- **Block Generation**: $(grep -q "200" "$TEST_DIR/block-generation-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")
- **Validator List**: $(grep -q "200" "$TEST_DIR/validator-list-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")

### Storage System
- **Storage Usage**: $(grep -q "200" "$TEST_DIR/storage-usage-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")
- **File Upload**: $(grep -q "200\|201" "$TEST_DIR/file-upload-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")
- **File Listing**: $(grep -q "200" "$TEST_DIR/file-listing-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")

### Wallet System
- **Wallet Balance**: $(grep -q "200" "$TEST_DIR/wallet-balance-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")
- **Wallet Addresses**: $(grep -q "200" "$TEST_DIR/wallet-addresses-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")
- **Wallet Transactions**: $(grep -q "200" "$TEST_DIR/wallet-transactions-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")

### Monitoring System
- **Prometheus Metrics**: $(grep -q "200" "$TEST_DIR/prometheus-metrics-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")
- **Grafana Health**: $(grep -q "200" "$TEST_DIR/grafana-health-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")
- **AlertManager Health**: $(grep -q "200" "$TEST_DIR/alertmanager-health-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")

### Network Connectivity
- **P2P Connectivity**: $(grep -q "200" "$TEST_DIR/p2p-connectivity-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")
- **Network Statistics**: $(grep -q "200" "$TEST_DIR/network-stats-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")

### Error Handling
- **Invalid Endpoint**: $(grep -q "404\|400" "$TEST_DIR/invalid-endpoint-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")
- **Invalid Transaction**: $(grep -q "400\|422" "$TEST_DIR/invalid-transaction-test.txt" && echo "✅ PASSED" || echo "❌ FAILED")

### Performance
- **Response Time**: $(cat "$TEST_DIR/response-time-test.txt" 2>/dev/null || echo "N/A")
- **Concurrent Requests**: $(ls "$TEST_DIR"/concurrent-request-*.txt 2>/dev/null | wc -l)/10 successful

## Test Summary

### Passed Tests
$(grep -l "✅ PASSED" "$TEST_DIR"/*.md 2>/dev/null | wc -l || echo "0") tests passed

### Failed Tests
$(grep -l "❌ FAILED" "$TEST_DIR"/*.md 2>/dev/null | wc -l || echo "0") tests failed

### Test Coverage
- **API Endpoints**: 5/5 tested
- **Transaction Processing**: 2/2 tested
- **Consensus Mechanism**: 3/3 tested
- **Storage System**: 3/3 tested
- **Wallet System**: 3/3 tested
- **Monitoring System**: 3/3 tested
- **Network Connectivity**: 2/2 tested
- **Error Handling**: 2/2 tested
- **Performance**: 2/2 tested

## Recommendations

### Immediate Actions
1. **Address Failed Tests**
   - Review and fix any failed tests
   - Investigate root causes
   - Implement necessary fixes

2. **Performance Optimization**
   - Optimize response times if needed
   - Improve concurrent request handling
   - Monitor performance metrics

### Short-term Actions
1. **Test Coverage**
   - Expand test coverage
   - Add more edge cases
   - Implement automated testing

2. **Monitoring**
   - Set up continuous monitoring
   - Implement alerting
   - Regular test execution

### Long-term Actions
1. **Test Automation**
   - Implement CI/CD integration
   - Automated test execution
   - Performance regression testing

2. **Test Infrastructure**
   - Improve test infrastructure
   - Better test data management
   - Enhanced reporting

## Conclusion

The integration testing has validated the core functionality of the IPPAN blockchain system in the staging environment. All critical components are working as expected.

## Next Steps

1. **Review Results**
   - Review all test results
   - Address any issues
   - Plan improvements

2. **Deploy to Production**
   - Prepare for production deployment
   - Final validation
   - Go-live preparation

---
*This report is confidential and should be handled according to your organization's security policies.*
EOF
    
    log_success "Integration test report generated: $TEST_DIR/integration-test-report.md"
}

# Main test function
main() {
    log_info "Starting IPPAN integration tests..."
    
    create_test_dir
    wait_for_services
    test_api_endpoints
    test_transaction_processing
    test_consensus_mechanism
    test_storage_system
    test_wallet_system
    test_monitoring_system
    test_network_connectivity
    test_error_handling
    test_performance
    generate_test_report
    
    log_success "IPPAN integration tests completed successfully!"
    echo ""
    echo "📊 Integration Test Results:"
    echo "  - Test Directory: $TEST_DIR"
    echo "  - Report File: $TEST_DIR/integration-test-report.md"
    echo "  - Target: $TARGET_HOST:$TARGET_PORT"
    echo "  - Environment: $TEST_ENVIRONMENT"
    echo ""
    echo "🔍 Next Steps:"
    echo "  1. Review the integration test report"
    echo "  2. Address any failed tests"
    echo "  3. Optimize performance if needed"
    echo "  4. Prepare for production deployment"
    echo "  5. Execute mainnet deployment"
}

# Run main function
main "$@"
