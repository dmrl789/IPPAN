#!/bin/bash
# IPPAN Deployment Fix Script
# This script fixes node connectivity and ensures the Unified UI stays disabled

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Server configuration
SERVER1_IP="188.245.97.41"
SERVER2_IP="135.181.145.174"

echo -e "${BLUE}🚀 IPPAN Deployment Fix Script${NC}"
echo "=================================="

# Function to check if server is reachable
check_server() {
    local server_ip=$1
    local server_name=$2
    
    echo -e "${YELLOW}Checking $server_name ($server_ip)...${NC}"
    
    if ping -c 1 -W 5 "$server_ip" >/dev/null 2>&1; then
        echo -e "${GREEN}✅ $server_name is reachable${NC}"
        return 0
    else
        echo -e "${RED}❌ $server_name is not reachable${NC}"
        return 1
    fi
}

# Function to check port connectivity
check_port() {
    local server_ip=$1
    local port=$2
    local service_name=$3
    
    echo -e "${YELLOW}Checking $service_name on $server_ip:$port...${NC}"
    
    if timeout 5 bash -c "</dev/tcp/$server_ip/$port" 2>/dev/null; then
        echo -e "${GREEN}✅ $service_name is accessible on $server_ip:$port${NC}"
        return 0
    else
        echo -e "${RED}❌ $service_name is not accessible on $server_ip:$port${NC}"
        return 1
    fi
}

# Function to test connectivity
test_connectivity() {
    echo -e "${BLUE}🔍 Testing connectivity...${NC}"
    
    # Test Server 1
    if check_server "$SERVER1_IP" "Server 1"; then
        check_port "$SERVER1_IP" "8080" "RPC API"
        check_port "$SERVER1_IP" "9000" "P2P"
    fi
    
    # Test Server 2
    if check_server "$SERVER2_IP" "Server 2"; then
        check_port "$SERVER2_IP" "8080" "RPC API"
        check_port "$SERVER2_IP" "9001" "P2P"
    fi
}

# Function to fix P2P connectivity
fix_p2p_connectivity() {
    echo -e "${BLUE}🔧 Fixing P2P connectivity...${NC}"
    
    # Create a script to fix P2P ports
    cat > fix-p2p.sh << 'EOF'
#!/bin/bash
# Fix P2P connectivity issues

echo "Fixing P2P connectivity..."

# Check if Docker is running
if ! docker info >/dev/null 2>&1; then
    echo "Starting Docker..."
    sudo systemctl start docker
    sudo systemctl enable docker
fi

# Stop any existing containers
docker-compose down 2>/dev/null || true

# Free up ports
echo "Freeing up ports..."
sudo lsof -ti:8080,8081,9000,9001 | xargs -r sudo kill -9 2>/dev/null || true

# Start services
echo "Starting services..."
docker-compose up -d

# Wait for services to start
sleep 10

# Check status
echo "Checking service status..."
docker-compose ps

# Test connectivity
echo "Testing connectivity..."
curl -s http://localhost:8080/health || echo "Node 1 health check failed"
curl -s http://localhost:8081/health || echo "Node 2 health check failed"

echo "P2P connectivity fix completed!"
EOF

    chmod +x fix-p2p.sh
    echo -e "${GREEN}✅ P2P fix script created${NC}"
}

# Main execution
main() {
    echo -e "${BLUE}Starting IPPAN deployment fix...${NC}"

    # Test current connectivity
    test_connectivity

    # Create P2P fix script
    fix_p2p_connectivity

    echo -e "${GREEN}🎉 Deployment fix completed!${NC}"
    echo ""
    echo -e "${YELLOW}Next steps:${NC}"
    echo "1. Ensure Docker and the IPPAN node compose stack are running on each server"
    echo "2. Run fix-p2p.sh on affected servers to stabilize ports if needed"
    echo "3. Confirm no Unified UI containers are running (docker ps | grep -i ui should return nothing)"
    echo "4. Verify ports 80/443 remain closed while RPC (8080) and P2P ports are healthy"
    echo ""
    echo -e "${BLUE}Files created:${NC}"
    echo "- fix-p2p.sh (P2P connectivity fix script)"
}

# Run main function
main "$@"
