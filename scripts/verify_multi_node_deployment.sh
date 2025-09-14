#!/bin/bash
set -e

# IPPAN Multi-Node Deployment Verification Script
# This script verifies server2 deployment and establishes connection to server1

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Server configuration
SERVER1_IP="188.245.97.41"    # Nuremberg
SERVER2_IP="135.181.145.174"  # Helsinki
IPPAN_USER="ippan"
IPPAN_PORT="8080"
API_PORT="3000"
METRICS_PORT="9090"

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_header() {
    echo -e "${BLUE}[HEADER]${NC} $1"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to test network connectivity
test_connectivity() {
    local target_ip=$1
    local port=$2
    local service_name=$3
    
    print_status "Testing connectivity to $service_name at $target_ip:$port"
    
    if timeout 10 bash -c "</dev/tcp/$target_ip/$port" 2>/dev/null; then
        print_status "✅ $service_name is reachable at $target_ip:$port"
        return 0
    else
        print_error "❌ $service_name is not reachable at $target_ip:$port"
        return 1
    fi
}

# Function to check service status via API
check_service_status() {
    local server_ip=$1
    local server_name=$2
    
    print_status "Checking $server_name service status..."
    
    # Test API endpoint
    if curl -s --connect-timeout 10 "http://$server_ip:$API_PORT/health" >/dev/null 2>&1; then
        print_status "✅ $server_name API is responding"
        
        # Get node info
        local node_info=$(curl -s --connect-timeout 10 "http://$server_ip:$API_PORT/api/v1/node/info" 2>/dev/null || echo "{}")
        if [ "$node_info" != "{}" ]; then
            print_status "Node info: $node_info"
        fi
        
        # Get network peers
        local peers=$(curl -s --connect-timeout 10 "http://$server_ip:$API_PORT/api/v1/network/peers" 2>/dev/null || echo "[]")
        if [ "$peers" != "[]" ]; then
            print_status "Connected peers: $peers"
        fi
        
        return 0
    else
        print_error "❌ $server_name API is not responding"
        return 1
    fi
}

# Function to check Docker services
check_docker_services() {
    local server_ip=$1
    local server_name=$2
    
    print_status "Checking Docker services on $server_name..."
    
    # Check if we can SSH to the server
    if ! ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no $IPPAN_USER@$server_ip "echo 'SSH connection successful'" 2>/dev/null; then
        print_error "❌ Cannot SSH to $server_name"
        return 1
    fi
    
    # Check Docker services
    local docker_status=$(ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no $IPPAN_USER@$server_ip "docker ps --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'" 2>/dev/null)
    
    if [ $? -eq 0 ]; then
        print_status "✅ Docker services on $server_name:"
        echo "$docker_status"
        return 0
    else
        print_error "❌ Cannot check Docker services on $server_name"
        return 1
    fi
}

# Function to verify IPPAN node configuration
verify_node_config() {
    local server_ip=$1
    local server_name=$2
    local config_file=$3
    
    print_status "Verifying IPPAN configuration on $server_name..."
    
    # Check if config file exists
    if ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no $IPPAN_USER@$server_ip "test -f $config_file" 2>/dev/null; then
        print_status "✅ Configuration file exists: $config_file"
        
        # Display relevant config sections
        local network_config=$(ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no $IPPAN_USER@$server_ip "grep -A 10 '\[network\]' $config_file" 2>/dev/null)
        if [ ! -z "$network_config" ]; then
            print_status "Network configuration:"
            echo "$network_config"
        fi
        
        return 0
    else
        print_error "❌ Configuration file not found: $config_file"
        return 1
    fi
}

# Function to test consensus participation
test_consensus() {
    local server_ip=$1
    local server_name=$2
    
    print_status "Testing consensus participation on $server_name..."
    
    # Get latest block info
    local block_info=$(curl -s --connect-timeout 10 "http://$server_ip:$API_PORT/api/v1/blockchain/latest" 2>/dev/null)
    
    if [ "$block_info" != "" ] && [ "$block_info" != "null" ]; then
        print_status "✅ $server_name is participating in consensus"
        print_status "Latest block info: $block_info"
        return 0
    else
        print_warning "⚠️  $server_name consensus status unclear"
        return 1
    fi
}

# Function to check monitoring endpoints
check_monitoring() {
    local server_ip=$1
    local server_name=$2
    
    print_status "Checking monitoring endpoints on $server_name..."
    
    # Check Prometheus metrics
    if curl -s --connect-timeout 10 "http://$server_ip:$METRICS_PORT/metrics" >/dev/null 2>&1; then
        print_status "✅ Prometheus metrics available on $server_name"
    else
        print_warning "⚠️  Prometheus metrics not available on $server_name"
    fi
    
    # Check Grafana (if accessible)
    if curl -s --connect-timeout 10 "http://$server_ip:3001" >/dev/null 2>&1; then
        print_status "✅ Grafana dashboard accessible on $server_name"
    else
        print_warning "⚠️  Grafana dashboard not accessible on $server_name"
    fi
}

# Main verification function
main() {
    print_header "🚀 IPPAN Multi-Node Deployment Verification"
    echo "Server 1 (Nuremberg): $SERVER1_IP"
    echo "Server 2 (Helsinki): $SERVER2_IP"
    echo "================================================"
    
    local server1_ok=true
    local server2_ok=true
    
    # Verify Server 1
    print_header "🔍 Verifying Server 1 (Nuremberg) - $SERVER1_IP"
    
    if ! test_connectivity $SERVER1_IP $IPPAN_PORT "IPPAN P2P"; then
        server1_ok=false
    fi
    
    if ! test_connectivity $SERVER1_IP $API_PORT "IPPAN API"; then
        server1_ok=false
    fi
    
    if ! check_service_status $SERVER1_IP "Server 1"; then
        server1_ok=false
    fi
    
    if ! check_docker_services $SERVER1_IP "Server 1"; then
        server1_ok=false
    fi
    
    if ! verify_node_config $SERVER1_IP "Server 1" "/opt/ippan/mainnet/config.toml"; then
        server1_ok=false
    fi
    
    check_monitoring $SERVER1_IP "Server 1"
    
    echo ""
    
    # Verify Server 2
    print_header "🔍 Verifying Server 2 (Helsinki) - $SERVER2_IP"
    
    if ! test_connectivity $SERVER2_IP $IPPAN_PORT "IPPAN P2P"; then
        server2_ok=false
    fi
    
    if ! test_connectivity $SERVER2_IP $API_PORT "IPPAN API"; then
        server2_ok=false
    fi
    
    if ! check_service_status $SERVER2_IP "Server 2"; then
        server2_ok=false
    fi
    
    if ! check_docker_services $SERVER2_IP "Server 2"; then
        server2_ok=false
    fi
    
    if ! verify_node_config $SERVER2_IP "Server 2" "/opt/ippan/mainnet/config.toml"; then
        server2_ok=false
    fi
    
    check_monitoring $SERVER2_IP "Server 2"
    
    echo ""
    
    # Test inter-node connectivity
    print_header "🔗 Testing Inter-Node Connectivity"
    
    print_status "Testing Server 1 → Server 2 connectivity"
    if test_connectivity $SERVER2_IP $IPPAN_PORT "Server 2 from Server 1"; then
        print_status "✅ Server 1 can reach Server 2"
    else
        print_error "❌ Server 1 cannot reach Server 2"
        server1_ok=false
    fi
    
    print_status "Testing Server 2 → Server 1 connectivity"
    if test_connectivity $SERVER1_IP $IPPAN_PORT "Server 1 from Server 2"; then
        print_status "✅ Server 2 can reach Server 1"
    else
        print_error "❌ Server 2 cannot reach Server 1"
        server2_ok=false
    fi
    
    echo ""
    
    # Test consensus participation
    print_header "⛓️  Testing Consensus Participation"
    
    test_consensus $SERVER1_IP "Server 1"
    test_consensus $SERVER2_IP "Server 2"
    
    echo ""
    
    # Final status report
    print_header "📊 Final Status Report"
    
    if [ "$server1_ok" = true ]; then
        print_status "✅ Server 1 (Nuremberg) is operational"
    else
        print_error "❌ Server 1 (Nuremberg) has issues"
    fi
    
    if [ "$server2_ok" = true ]; then
        print_status "✅ Server 2 (Helsinki) is operational"
    else
        print_error "❌ Server 2 (Helsinki) has issues"
    fi
    
    if [ "$server1_ok" = true ] && [ "$server2_ok" = true ]; then
        print_status "🎉 Multi-node IPPAN network is operational!"
        print_status "Both servers are connected and participating in consensus"
        
        echo ""
        print_status "Access URLs:"
        echo "  Server 1 API: http://$SERVER1_IP:$API_PORT"
        echo "  Server 2 API: http://$SERVER2_IP:$API_PORT"
        echo "  Server 1 Grafana: http://$SERVER1_IP:3001"
        echo "  Server 2 Grafana: http://$SERVER2_IP:3001"
        echo "  Server 1 Prometheus: http://$SERVER1_IP:$METRICS_PORT"
        echo "  Server 2 Prometheus: http://$SERVER2_IP:$METRICS_PORT"
        
    else
        print_error "⚠️  Multi-node network has issues that need to be resolved"
        print_status "Please check the error messages above and fix the issues"
    fi
}

# Check prerequisites
if ! command_exists curl; then
    print_error "curl is required but not installed"
    exit 1
fi

if ! command_exists ssh; then
    print_error "ssh is required but not installed"
    exit 1
fi

# Run main function
main "$@"
