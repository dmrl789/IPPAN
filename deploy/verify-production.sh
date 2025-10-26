#!/bin/bash

# IPPAN Production Deployment Verification Script
# Verifies that all production components are properly deployed and configured

set -e

# Configuration
NODE1_URL="http://188.245.97.41:8080"
NODE2_URL="http://135.181.145.174:8080"
TIMEOUT=15

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0
WARNING_CHECKS=0

# Function to print colored output
print_check() {
    local status=$1
    local message=$2
    local details=$3
    
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    case $status in
        "PASS")
            echo -e "${GREEN}✓${NC} $message"
            if [ -n "$details" ]; then
                echo -e "    ${GREEN}→${NC} $details"
            fi
            PASSED_CHECKS=$((PASSED_CHECKS + 1))
            ;;
        "FAIL")
            echo -e "${RED}✗${NC} $message"
            if [ -n "$details" ]; then
                echo -e "    ${RED}→${NC} $details"
            fi
            FAILED_CHECKS=$((FAILED_CHECKS + 1))
            ;;
        "WARN")
            echo -e "${YELLOW}⚠${NC} $message"
            if [ -n "$details" ]; then
                echo -e "    ${YELLOW}→${NC} $details"
            fi
            WARNING_CHECKS=$((WARNING_CHECKS + 1))
            ;;
        "INFO")
            echo -e "${BLUE}ℹ${NC} $message"
            if [ -n "$details" ]; then
                echo -e "    ${BLUE}→${NC} $details"
            fi
            ;;
    esac
}

# Function to check HTTP endpoint
check_endpoint() {
    local url=$1
    local endpoint=$2
    local expected_status=$3
    local check_name=$4
    
    local full_url="${url}${endpoint}"
    local response=$(curl -s -w "%{http_code}" -o /tmp/verify_response.json --connect-timeout $TIMEOUT "$full_url" 2>/dev/null || echo "000")
    
    if [ "$response" = "$expected_status" ]; then
        print_check "PASS" "$check_name" "HTTP $response"
        return 0
    else
        print_check "FAIL" "$check_name" "Expected HTTP $expected_status, got $response"
        return 1
    fi
}

# Function to check JSON field
check_json_field() {
    local url=$1
    local endpoint=$2
    local field=$3
    local expected_value=$4
    local check_name=$5
    
    local full_url="${url}${endpoint}"
    local response=$(curl -s --connect-timeout $TIMEOUT "$full_url" 2>/dev/null)
    
    if [ $? -eq 0 ] && echo "$response" | jq -e ".$field" >/dev/null 2>&1; then
        local value=$(echo "$response" | jq -r ".$field")
        if [ "$value" = "$expected_value" ]; then
            print_check "PASS" "$check_name" "$field = $value"
            return 0
        else
            print_check "WARN" "$check_name" "$field = $value (expected $expected_value)"
            return 1
        fi
    else
        print_check "FAIL" "$check_name" "Failed to get $field from $endpoint"
        return 1
    fi
}

# Function to check node basic health
check_node_basic_health() {
    local url=$1
    local node_name=$2
    
    echo "Checking $node_name basic health..."
    
    # Check health endpoint
    check_endpoint "$url" "/health" "200" "Health endpoint"
    
    # Check status endpoint
    check_endpoint "$url" "/status" "200" "Status endpoint"
    
    # Check time endpoint
    check_endpoint "$url" "/time" "200" "Time endpoint"
    
    # Check peers endpoint
    check_endpoint "$url" "/peers" "200" "Peers endpoint"
    
    # Check version endpoint
    check_endpoint "$url" "/version" "200" "Version endpoint"
    
    echo ""
}

# Function to check node advanced health
check_node_advanced_health() {
    local url=$1
    local node_name=$2
    
    echo "Checking $node_name advanced health..."
    
    # Get health data
    local health_response=$(curl -s --connect-timeout $TIMEOUT "${url}/health" 2>/dev/null)
    
    if [ $? -eq 0 ]; then
        # Check node status
        local status=$(echo "$health_response" | jq -r '.status // "unknown"' 2>/dev/null || echo "unknown")
        if [ "$status" = "ok" ]; then
            print_check "PASS" "Node status" "Status: $status"
        else
            print_check "FAIL" "Node status" "Status: $status"
        fi
        
        # Check peer count
        local peer_count=$(echo "$health_response" | jq -r '.peer_count // 0' 2>/dev/null || echo "0")
        if [ "$peer_count" -gt 0 ]; then
            print_check "PASS" "Peer connectivity" "Connected to $peer_count peers"
        else
            print_check "WARN" "Peer connectivity" "No peer connections"
        fi
        
        # Check mempool
        local mempool_size=$(echo "$health_response" | jq -r '.mempool_size // 0' 2>/dev/null || echo "0")
        print_check "INFO" "Mempool status" "Size: $mempool_size transactions"
        
        # Check uptime
        local uptime=$(echo "$health_response" | jq -r '.uptime_secs // 0' 2>/dev/null || echo "0")
        if [ "$uptime" -gt 60 ]; then
            print_check "PASS" "Node uptime" "${uptime}s"
        else
            print_check "WARN" "Node uptime" "${uptime}s (may be restarting)"
        fi
        
        # Check consensus if available
        if echo "$health_response" | jq -e '.consensus' >/dev/null 2>&1; then
            local latest_height=$(echo "$health_response" | jq -r '.consensus.latest_block_height // 0' 2>/dev/null || echo "0")
            local current_slot=$(echo "$health_response" | jq -r '.consensus.current_slot // 0' 2>/dev/null || echo "0")
            local validator_count=$(echo "$health_response" | jq -r '.consensus.validator_count // 0' 2>/dev/null || echo "0")
            
            print_check "INFO" "Consensus status" "Height: $latest_height, Slot: $current_slot, Validators: $validator_count"
        fi
    else
        print_check "FAIL" "Health data retrieval" "Failed to get health data"
    fi
    
    echo ""
}

# Function to check network connectivity
check_network_connectivity() {
    echo "Checking network connectivity..."
    
    # Check Node 1 reachability
    if curl -s --connect-timeout $TIMEOUT "$NODE1_URL/health" >/dev/null 2>&1; then
        print_check "PASS" "Node 1 reachability" "188.245.97.41:8080"
    else
        print_check "FAIL" "Node 1 reachability" "188.245.97.41:8080"
    fi
    
    # Check Node 2 reachability
    if curl -s --connect-timeout $TIMEOUT "$NODE2_URL/health" >/dev/null 2>&1; then
        print_check "PASS" "Node 2 reachability" "135.181.145.174:8080"
    else
        print_check "FAIL" "Node 2 reachability" "135.181.145.174:8080"
    fi
    
    echo ""
}

# Function to check consensus synchronization
check_consensus_sync() {
    echo "Checking consensus synchronization..."
    
    local node1_health=$(curl -s --connect-timeout $TIMEOUT "${NODE1_URL}/health" 2>/dev/null)
    local node2_health=$(curl -s --connect-timeout $TIMEOUT "${NODE2_URL}/health" 2>/dev/null)
    
    if [ $? -eq 0 ] && echo "$node1_health" | jq -e '.consensus' >/dev/null 2>&1 && echo "$node2_health" | jq -e '.consensus' >/dev/null 2>&1; then
        local node1_height=$(echo "$node1_health" | jq -r '.consensus.latest_block_height // 0' 2>/dev/null || echo "0")
        local node2_height=$(echo "$node2_health" | jq -r '.consensus.latest_block_height // 0' 2>/dev/null || echo "0")
        
        if [ "$node1_height" -gt 0 ] && [ "$node2_height" -gt 0 ]; then
            local height_diff=$((node1_height - node2_height))
            if [ ${height_diff#-} -le 5 ]; then
                print_check "PASS" "Block synchronization" "Height diff: $height_diff (Node 1: $node1_height, Node 2: $node2_height)"
            else
                print_check "WARN" "Block synchronization" "Height diff: $height_diff (Node 1: $node1_height, Node 2: $node2_height)"
            fi
        else
            print_check "WARN" "Block synchronization" "One or both nodes have no blocks"
        fi
    else
        print_check "FAIL" "Consensus synchronization" "Failed to get consensus data"
    fi
    
    echo ""
}

# Function to check API functionality
check_api_functionality() {
    echo "Checking API functionality..."
    
    # Test transaction submission (this would fail but should return proper error)
    local tx_response=$(curl -s -w "%{http_code}" -o /tmp/tx_response.json -X POST \
        -H "Content-Type: application/json" \
        -d '{"from":"0000000000000000000000000000000000000000000000000000000000000001","to":"0000000000000000000000000000000000000000000000000000000000000002","amount":1000,"nonce":1,"signature":"00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"}' \
        --connect-timeout $TIMEOUT "$NODE1_URL/tx" 2>/dev/null || echo "000")
    
    if [ "$tx_response" = "200" ] || [ "$tx_response" = "400" ] || [ "$tx_response" = "422" ]; then
        print_check "PASS" "Transaction API" "Endpoint responding (HTTP $tx_response)"
    else
        print_check "FAIL" "Transaction API" "Endpoint not responding (HTTP $tx_response)"
    fi
    
    # Test block retrieval
    local block_response=$(curl -s -w "%{http_code}" -o /tmp/block_response.json --connect-timeout $TIMEOUT "$NODE1_URL/block/0" 2>/dev/null || echo "000")
    
    if [ "$block_response" = "200" ] || [ "$block_response" = "404" ]; then
        print_check "PASS" "Block API" "Endpoint responding (HTTP $block_response)"
    else
        print_check "FAIL" "Block API" "Endpoint not responding (HTTP $block_response)"
    fi
    
    echo ""
}

# Function to print summary
print_summary() {
    echo ""
    echo "Verification Summary"
    echo "==================="
    echo "Total checks: $TOTAL_CHECKS"
    echo -e "Passed: ${GREEN}$PASSED_CHECKS${NC}"
    echo -e "Warnings: ${YELLOW}$WARNING_CHECKS${NC}"
    echo -e "Failed: ${RED}$FAILED_CHECKS${NC}"
    echo ""
    
    if [ $FAILED_CHECKS -eq 0 ]; then
        if [ $WARNING_CHECKS -eq 0 ]; then
            echo -e "${GREEN}✓ All checks passed! Production deployment is healthy.${NC}"
            exit 0
        else
            echo -e "${YELLOW}⚠ Some warnings detected, but deployment is functional.${NC}"
            exit 0
        fi
    else
        echo -e "${RED}✗ Some checks failed. Please review the issues above.${NC}"
        exit 1
    fi
}

# Main execution
main() {
    echo "IPPAN Production Deployment Verification"
    echo "======================================="
    echo ""
    
    # Run all checks
    check_network_connectivity
    check_node_basic_health "$NODE1_URL" "Node 1 (188.245.97.41)"
    check_node_basic_health "$NODE2_URL" "Node 2 (135.181.145.174)"
    check_node_advanced_health "$NODE1_URL" "Node 1 (188.245.97.41)"
    check_node_advanced_health "$NODE2_URL" "Node 2 (135.181.145.174)"
    check_consensus_sync
    check_api_functionality
    
    # Print summary
    print_summary
}

# Run main function
main "$@"