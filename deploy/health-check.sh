#!/bin/bash

# IPPAN Production Health Check Script
# This script performs comprehensive health checks on IPPAN nodes

set -e

# Configuration
NODE1_URL="http://188.245.97.41:8080"
NODE2_URL="http://135.181.145.174:8080"
TIMEOUT=10

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    local status=$1
    local message=$2
    local node=$3
    
    if [ "$status" = "OK" ]; then
        echo -e "${GREEN}✓${NC} $node: $message"
    elif [ "$status" = "WARN" ]; then
        echo -e "${YELLOW}⚠${NC} $node: $message"
    else
        echo -e "${RED}✗${NC} $node: $message"
    fi
}

# Function to check HTTP endpoint
check_endpoint() {
    local url=$1
    local endpoint=$2
    local node=$3
    
    local full_url="${url}${endpoint}"
    local response=$(curl -s -w "%{http_code}" -o /tmp/health_response.json --connect-timeout $TIMEOUT "$full_url" 2>/dev/null || echo "000")
    
    if [ "$response" = "200" ]; then
        print_status "OK" "Endpoint $endpoint responding" "$node"
        return 0
    else
        print_status "ERROR" "Endpoint $endpoint returned $response" "$node"
        return 1
    fi
}

# Function to check JSON response
check_json_response() {
    local url=$1
    local endpoint=$2
    local node=$3
    local field=$4
    local expected_value=$5
    
    local full_url="${url}${endpoint}"
    local response=$(curl -s --connect-timeout $TIMEOUT "$full_url" 2>/dev/null)
    
    if [ $? -eq 0 ] && echo "$response" | jq -e ".$field" >/dev/null 2>&1; then
        local value=$(echo "$response" | jq -r ".$field")
        if [ "$value" = "$expected_value" ]; then
            print_status "OK" "Field $field = $value" "$node"
        else
            print_status "WARN" "Field $field = $value (expected $expected_value)" "$node"
        fi
    else
        print_status "ERROR" "Failed to parse JSON response from $endpoint" "$node"
    fi
}

# Function to check node health
check_node_health() {
    local url=$1
    local node=$2
    
    echo "Checking $node ($url)..."
    
    # Check basic health endpoint
    check_endpoint "$url" "/health" "$node"
    
    # Check consensus status
    check_endpoint "$url" "/status" "$node"
    
    # Check time service
    check_endpoint "$url" "/time" "$node"
    
    # Check peer connectivity
    check_endpoint "$url" "/peers" "$node"
    
    # Check if node is proposing blocks
    local health_response=$(curl -s --connect-timeout $TIMEOUT "${url}/health" 2>/dev/null)
    if [ $? -eq 0 ]; then
        local peer_count=$(echo "$health_response" | jq -r '.peer_count // 0' 2>/dev/null || echo "0")
        local mempool_size=$(echo "$health_response" | jq -r '.mempool_size // 0' 2>/dev/null || echo "0")
        local uptime=$(echo "$health_response" | jq -r '.uptime_secs // 0' 2>/dev/null || echo "0")
        
        if [ "$peer_count" -gt 0 ]; then
            print_status "OK" "Connected to $peer_count peers" "$node"
        else
            print_status "WARN" "No peer connections" "$node"
        fi
        
        if [ "$mempool_size" -gt 0 ]; then
            print_status "OK" "Mempool has $mempool_size transactions" "$node"
        else
            print_status "WARN" "Mempool is empty" "$node"
        fi
        
        if [ "$uptime" -gt 60 ]; then
            print_status "OK" "Node uptime: ${uptime}s" "$node"
        else
            print_status "WARN" "Node uptime: ${uptime}s (may be restarting)" "$node"
        fi
    fi
    
    echo ""
}

# Function to check network connectivity
check_network_connectivity() {
    echo "Checking network connectivity..."
    
    # Check if nodes can reach each other
    if curl -s --connect-timeout $TIMEOUT "$NODE1_URL/health" >/dev/null 2>&1; then
        print_status "OK" "Node 1 is reachable" "Network"
    else
        print_status "ERROR" "Node 1 is not reachable" "Network"
    fi
    
    if curl -s --connect-timeout $TIMEOUT "$NODE2_URL/health" >/dev/null 2>&1; then
        print_status "OK" "Node 2 is reachable" "Network"
    else
        print_status "ERROR" "Node 2 is not reachable" "Network"
    fi
    
    echo ""
}

# Function to check consensus health
check_consensus_health() {
    echo "Checking consensus health..."
    
    # Check if both nodes are producing blocks
    local node1_health=$(curl -s --connect-timeout $TIMEOUT "${NODE1_URL}/health" 2>/dev/null)
    local node2_health=$(curl -s --connect-timeout $TIMEOUT "${NODE2_URL}/health" 2>/dev/null)
    
    if [ $? -eq 0 ] && echo "$node1_health" | jq -e '.consensus' >/dev/null 2>&1; then
        local node1_height=$(echo "$node1_health" | jq -r '.consensus.latest_block_height // 0' 2>/dev/null || echo "0")
        local node2_height=$(echo "$node2_health" | jq -r '.consensus.latest_block_height // 0' 2>/dev/null || echo "0")
        
        if [ "$node1_height" -gt 0 ]; then
            print_status "OK" "Node 1 block height: $node1_height" "Consensus"
        else
            print_status "WARN" "Node 1 has no blocks" "Consensus"
        fi
        
        if [ "$node2_height" -gt 0 ]; then
            print_status "OK" "Node 2 block height: $node2_height" "Consensus"
        else
            print_status "WARN" "Node 2 has no blocks" "Consensus"
        fi
        
        # Check if heights are reasonably close (within 5 blocks)
        local height_diff=$((node1_height - node2_height))
        if [ ${height_diff#-} -le 5 ]; then
            print_status "OK" "Nodes are synchronized (height diff: $height_diff)" "Consensus"
        else
            print_status "WARN" "Nodes may be out of sync (height diff: $height_diff)" "Consensus"
        fi
    else
        print_status "ERROR" "Failed to get consensus information" "Consensus"
    fi
    
    echo ""
}

# Main execution
main() {
    echo "IPPAN Production Health Check"
    echo "============================="
    echo ""
    
    # Check network connectivity
    check_network_connectivity
    
    # Check individual nodes
    check_node_health "$NODE1_URL" "Node 1 (188.245.97.41)"
    check_node_health "$NODE2_URL" "Node 2 (135.181.145.174)"
    
    # Check consensus health
    check_consensus_health
    
    echo "Health check completed."
}

# Run main function
main "$@"