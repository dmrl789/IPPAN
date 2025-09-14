#!/bin/bash
set -e

# IPPAN Server Status Check and Connection Script
# This script checks the status of server1 and server2 and establishes connections

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Server configuration
SERVER1_IP="188.245.97.41"    # Nuremberg (Node 1)
SERVER2_IP="135.181.145.174"  # Helsinki (Node 2)
IPPAN_USER="ippan"

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

print_section() {
    echo -e "\n${CYAN}=== $1 ===${NC}"
}

# Function to test basic connectivity
test_server_connectivity() {
    local server_ip="$1"
    local server_name="$2"
    
    print_header "🔍 Testing $server_name Connectivity ($server_ip)"
    
    # Test ping
    print_status "Testing ping connectivity..."
    if ping -c 2 -W 5 "$server_ip" >/dev/null 2>&1; then
        print_status "✅ $server_name is reachable via ping"
    else
        print_error "❌ $server_name is not reachable via ping"
        return 1
    fi
    
    # Test SSH connectivity
    print_status "Testing SSH connectivity..."
    if ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no -o BatchMode=yes "$IPPAN_USER@$server_ip" "echo 'SSH connection successful'" >/dev/null 2>&1; then
        print_status "✅ $server_name SSH connection successful"
    else
        print_warning "⚠️  $server_name SSH connection failed or requires authentication"
    fi
    
    return 0
}

# Function to check server services
test_server_services() {
    local server_ip="$1"
    local server_name="$2"
    
    print_header "🔧 Checking $server_name Services"
    
    # Test API endpoint
    print_status "Testing API endpoint (port 3000)..."
    if curl -s --connect-timeout 10 "http://$server_ip:3000/health" >/dev/null 2>&1; then
        print_status "✅ $server_name API is responding"
        local api_response=$(curl -s --connect-timeout 10 "http://$server_ip:3000/health" 2>/dev/null)
        print_status "API Response: $api_response"
    else
        print_warning "⚠️  $server_name API is not responding"
    fi
    
    # Test P2P port
    print_status "Testing P2P port (8080)..."
    if timeout 10 bash -c "</dev/tcp/$server_ip/8080" 2>/dev/null; then
        print_status "✅ $server_name P2P port (8080) is open"
    else
        print_warning "⚠️  $server_name P2P port (8080) connection timeout"
    fi
    
    # Test Prometheus metrics port
    print_status "Testing Prometheus metrics port (9090)..."
    if curl -s --connect-timeout 10 "http://$server_ip:9090/metrics" >/dev/null 2>&1; then
        print_status "✅ $server_name Prometheus metrics are available"
    else
        print_warning "⚠️  $server_name Prometheus metrics are not available"
    fi
    
    # Test Grafana dashboard port
    print_status "Testing Grafana dashboard port (3001)..."
    if curl -s --connect-timeout 10 "http://$server_ip:3001" >/dev/null 2>&1; then
        print_status "✅ $server_name Grafana dashboard is accessible"
    else
        print_warning "⚠️  $server_name Grafana dashboard is not accessible"
    fi
}

# Function to check Docker containers on server
test_server_docker_containers() {
    local server_ip="$1"
    local server_name="$2"
    
    print_header "🐳 Checking $server_name Docker Containers"
    
    local docker_ps=$(ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no "$IPPAN_USER@$server_ip" "docker ps --filter 'name=ippan' --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'" 2>/dev/null)
    if [ -n "$docker_ps" ]; then
        print_status "✅ $server_name Docker containers status:"
        echo "$docker_ps"
    else
        print_warning "⚠️  $server_name No IPPAN Docker containers found or SSH access failed"
    fi
}

# Function to check network peer connections
test_network_peer_connections() {
    local server_ip="$1"
    local server_name="$2"
    
    print_header "🔗 Checking $server_name Network Peer Connections"
    
    # Check peer list via API
    print_status "Checking peer list via API..."
    local peers_response=$(curl -s --connect-timeout 10 "http://$server_ip:3000/api/v1/network/peers" 2>/dev/null)
    if [ -n "$peers_response" ]; then
        print_status "✅ $server_name Peer list retrieved successfully"
        local peer_count=$(echo "$peers_response" | jq -r '.peers | length' 2>/dev/null || echo "unknown")
        print_status "Peer count: $peer_count"
        echo "$peers_response" | jq -r '.peers[]? | "  - Peer: \(.address) (Status: \(.status))"' 2>/dev/null || echo "  - Unable to parse peer data"
    else
        print_warning "⚠️  $server_name Peer list API not available"
    fi
    
    # Check blockchain status
    print_status "Checking blockchain status..."
    local blockchain_response=$(curl -s --connect-timeout 10 "http://$server_ip:3000/api/v1/blockchain/status" 2>/dev/null)
    if [ -n "$blockchain_response" ]; then
        print_status "✅ $server_name Blockchain status retrieved"
        local block_height=$(echo "$blockchain_response" | jq -r '.block_height' 2>/dev/null || echo "unknown")
        local network_status=$(echo "$blockchain_response" | jq -r '.network_status' 2>/dev/null || echo "unknown")
        print_status "Block height: $block_height"
        print_status "Network status: $network_status"
    else
        print_warning "⚠️  $server_name Blockchain status API not available"
    fi
}

# Function to test inter-server connectivity
test_inter_server_connectivity() {
    print_header "🔗 Testing Inter-Server Connectivity"
    
    # Test server1 to server2 connectivity
    print_status "Testing Server1 to Server2 connectivity..."
    if ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no "$IPPAN_USER@$SERVER1_IP" "timeout 10 bash -c '</dev/tcp/$SERVER2_IP/8080' && echo 'Server2 P2P reachable from Server1'" >/dev/null 2>&1; then
        print_status "✅ Server1 can reach Server2 P2P port"
    else
        print_warning "⚠️  Server1 cannot reach Server2 P2P port"
    fi
    
    # Test server2 to server1 connectivity
    print_status "Testing Server2 to Server1 connectivity..."
    if ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no "$IPPAN_USER@$SERVER2_IP" "timeout 10 bash -c '</dev/tcp/$SERVER1_IP/8080' && echo 'Server1 P2P reachable from Server2'" >/dev/null 2>&1; then
        print_status "✅ Server2 can reach Server1 P2P port"
    else
        print_warning "⚠️  Server2 cannot reach Server1 P2P port"
    fi
}

# Function to restart services if needed
restart_server_services() {
    local server_ip="$1"
    local server_name="$2"
    
    print_header "🔄 Restarting $server_name Services"
    
    print_status "Stopping IPPAN services..."
    ssh -o ConnectTimeout=30 -o StrictHostKeyChecking=no "$IPPAN_USER@$server_ip" "cd /opt/ippan/mainnet && docker-compose down" 2>/dev/null
    
    print_status "Starting IPPAN services..."
    ssh -o ConnectTimeout=30 -o StrictHostKeyChecking=no "$IPPAN_USER@$server_ip" "cd /opt/ippan/mainnet && docker-compose up -d" 2>/dev/null
    
    print_status "Waiting for services to start..."
    sleep 30
    
    print_status "Checking service status..."
    ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no "$IPPAN_USER@$server_ip" "cd /opt/ippan/mainnet && docker-compose ps" 2>/dev/null
    
    print_status "✅ $server_name services restarted"
}

# Function to create connection verification report
create_connection_report() {
    print_header "📊 Connection Verification Report"
    
    local report="IPPAN Multi-Node Connection Report
Generated: $(date)
=====================================

Server Configuration:
- Server 1 (Nuremberg): $SERVER1_IP
- Server 2 (Helsinki): $SERVER2_IP
- User: $IPPAN_USER

Access URLs:
- Server 1 API: http://$SERVER1_IP:3000
- Server 1 Grafana: http://$SERVER1_IP:3001
- Server 1 Prometheus: http://$SERVER1_IP:9090
- Server 2 API: http://$SERVER2_IP:3000
- Server 2 Grafana: http://$SERVER2_IP:3001
- Server 2 Prometheus: http://$SERVER2_IP:9090

Network Ports:
- P2P Network: 8080
- API: 3000
- Prometheus: 9090
- Grafana: 3001

Next Steps:
1. Monitor logs: docker-compose logs -f
2. Check consensus participation via API endpoints
3. Verify blockchain synchronization
4. Test transaction processing"
    
    echo "$report"
    echo "$report" > ippan_connection_report.txt
    print_status "Report saved to: ippan_connection_report.txt"
}

# Main execution
print_section "IPPAN Server Status Check and Connection Script"
print_header "🚀 Starting comprehensive server check and connection process"
echo "Server 1 (Nuremberg): $SERVER1_IP"
echo "Server 2 (Helsinki): $SERVER2_IP"
echo "================================================"

# Check server connectivity
server1_reachable=false
server2_reachable=false

if test_server_connectivity "$SERVER1_IP" "Server1"; then
    server1_reachable=true
fi

if test_server_connectivity "$SERVER2_IP" "Server2"; then
    server2_reachable=true
fi

if [ "$server1_reachable" = false ] && [ "$server2_reachable" = false ]; then
    print_error "❌ Neither server is reachable. Please check network connectivity and server status."
    exit 1
fi

# Check services on each server
if [ "$server1_reachable" = true ]; then
    test_server_services "$SERVER1_IP" "Server1"
    test_server_docker_containers "$SERVER1_IP" "Server1"
    test_network_peer_connections "$SERVER1_IP" "Server1"
fi

if [ "$server2_reachable" = true ]; then
    test_server_services "$SERVER2_IP" "Server2"
    test_server_docker_containers "$SERVER2_IP" "Server2"
    test_network_peer_connections "$SERVER2_IP" "Server2"
fi

# Test inter-server connectivity
if [ "$server1_reachable" = true ] && [ "$server2_reachable" = true ]; then
    test_inter_server_connectivity
fi

# Ask user if they want to restart services
print_section "Service Management"
echo -n "Do you want to restart services on both servers? (y/n): "
read -r restart_choice
if [ "$restart_choice" = "y" ] || [ "$restart_choice" = "Y" ]; then
    if [ "$server1_reachable" = true ]; then
        restart_server_services "$SERVER1_IP" "Server1"
    fi
    if [ "$server2_reachable" = true ]; then
        restart_server_services "$SERVER2_IP" "Server2"
    fi
    
    print_status "Waiting for services to stabilize..."
    sleep 60
    
    # Re-check services after restart
    if [ "$server1_reachable" = true ]; then
        test_server_services "$SERVER1_IP" "Server1"
    fi
    if [ "$server2_reachable" = true ]; then
        test_server_services "$SERVER2_IP" "Server2"
    fi
fi

# Generate connection report
create_connection_report

print_section "Check Complete"
print_status "🎉 Server status check and connection verification complete!"
print_status "Review the report above and check the generated file: ippan_connection_report.txt"
print_status ""
print_status "If services are not running properly, you can:"
print_status "1. Check Docker logs: ssh $IPPAN_USER@<server_ip> 'cd /opt/ippan/mainnet && docker-compose logs'"
print_status "2. Restart services: ssh $IPPAN_USER@<server_ip> 'cd /opt/ippan/mainnet && docker-compose restart'"
print_status "3. Check system resources: ssh $IPPAN_USER@<server_ip> 'docker stats'"
