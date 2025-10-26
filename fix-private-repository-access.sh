#!/bin/bash

# 🔧 IPPAN Private Repository Access Fix Script
# This script diagnoses and fixes connectivity issues with private IPPAN services

set -e

echo "🚀 Starting IPPAN Private Repository Access Fix..."
echo "=================================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Server configuration
SERVER_IP="188.245.97.41"
UI_PORT="3001"
API_PORT="8080"  # Corrected from 7080
GATEWAY_PORT="8081"

echo -e "${BLUE}📋 Diagnosing connectivity issues...${NC}"

# Function to test port connectivity
test_port() {
    local host=$1
    local port=$2
    local service_name=$3
    
    echo -n "Testing $service_name ($host:$port)... "
    
    if timeout 5 bash -c "cat < /dev/null > /dev/tcp/$host/$port" 2>/dev/null; then
        echo -e "${GREEN}✅ Accessible${NC}"
        return 0
    else
        echo -e "${RED}❌ Not accessible${NC}"
        return 1
    fi
}

# Test all ports
echo -e "\n${YELLOW}🔍 Port Connectivity Tests:${NC}"
test_port $SERVER_IP $UI_PORT "UI Service"
test_port $SERVER_IP $API_PORT "API Service" 
test_port $SERVER_IP $GATEWAY_PORT "Gateway Service"

echo -e "\n${YELLOW}🌐 Testing HTTP endpoints:${NC}"

# Test UI endpoint
echo -n "Testing UI endpoint (http://$SERVER_IP:$UI_PORT/)... "
if curl -sS -I "http://$SERVER_IP:$UI_PORT/" >/dev/null 2>&1; then
    echo -e "${GREEN}✅ Responding${NC}"
else
    echo -e "${RED}❌ Not responding${NC}"
fi

# Test API endpoint
echo -n "Testing API endpoint (http://$SERVER_IP:$API_PORT/health)... "
if curl -sS "http://$SERVER_IP:$API_PORT/health" >/dev/null 2>&1; then
    echo -e "${GREEN}✅ Responding${NC}"
else
    echo -e "${RED}❌ Not responding${NC}"
fi

# Test Gateway endpoint
echo -n "Testing Gateway endpoint (http://$SERVER_IP:$GATEWAY_PORT/health)... "
if curl -sS "http://$SERVER_IP:$GATEWAY_PORT/health" >/dev/null 2>&1; then
    echo -e "${GREEN}✅ Responding${NC}"
else
    echo -e "${RED}❌ Not responding${NC}"
fi

echo -e "\n${YELLOW}🔧 Recommended Fix Actions:${NC}"

# Check if we're on the server
if [[ "$(hostname -I)" == *"$SERVER_IP"* ]]; then
    echo -e "${BLUE}📍 Running on server $SERVER_IP - can perform direct fixes${NC}"
    
    # Check Docker status
    echo -e "\n${YELLOW}🐳 Checking Docker services:${NC}"
    if command -v docker >/dev/null 2>&1; then
        echo "Docker containers:"
        docker ps -a || echo "No Docker containers found"
        
        echo -e "\nDocker Compose status:"
        if [ -f "deploy/docker-compose.full-stack.yml" ]; then
            docker compose -f deploy/docker-compose.full-stack.yml ps || echo "No services running"
        fi
    else
        echo -e "${RED}❌ Docker not installed or not accessible${NC}"
    fi
    
    # Check systemd services
    echo -e "\n${YELLOW}🔧 Checking systemd services:${NC}"
    if systemctl is-active --quiet ippan-node 2>/dev/null; then
        echo -e "${GREEN}✅ ippan-node service is running${NC}"
    else
        echo -e "${RED}❌ ippan-node service is not running${NC}"
    fi
    
else
    echo -e "${BLUE}📍 Running remotely - providing diagnostic information${NC}"
fi

echo -e "\n${YELLOW}📋 Fix Recommendations:${NC}"

echo "1. ${BLUE}Check Docker Services:${NC}"
echo "   If on server, run:"
echo "   cd /path/to/ippan"
echo "   docker compose -f deploy/docker-compose.full-stack.yml up -d"
echo "   docker compose -f deploy/docker-compose.full-stack.yml ps"

echo -e "\n2. ${BLUE}Check Environment Configuration:${NC}"
echo "   Verify .env file contains:"
echo "   NEXT_PUBLIC_ENABLE_FULL_UI=1"
echo "   NEXT_PUBLIC_GATEWAY_URL=http://$SERVER_IP:$GATEWAY_PORT/api"
echo "   NEXT_PUBLIC_API_BASE_URL=http://$SERVER_IP:$API_PORT"
echo "   NEXT_PUBLIC_WS_URL=ws://$SERVER_IP:$API_PORT/ws"

echo -e "\n3. ${BLUE}Check Port Mappings:${NC}"
echo "   UI should be accessible on: http://$SERVER_IP:$UI_PORT"
echo "   API should be accessible on: http://$SERVER_IP:$API_PORT"
echo "   Gateway should be accessible on: http://$SERVER_IP:$GATEWAY_PORT"

echo -e "\n4. ${BLUE}Restart Services:${NC}"
echo "   If services are running but not responding:"
echo "   docker compose -f deploy/docker-compose.full-stack.yml restart"

echo -e "\n5. ${BLUE}Check Logs:${NC}"
echo "   docker compose -f deploy/docker-compose.full-stack.yml logs -f"

echo -e "\n${GREEN}✅ Diagnostic complete!${NC}"
echo -e "${BLUE}💡 Next step: Run the recommended commands above to fix the connectivity issues.${NC}"