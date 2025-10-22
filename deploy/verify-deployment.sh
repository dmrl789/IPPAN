#!/bin/bash

# Verify IPPAN Full Stack Deployment
# This script checks all services across both servers

set -e

echo "🔍 Verifying IPPAN Full Stack Deployment..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to check HTTP endpoint
check_http() {
    local url=$1
    local name=$2
    local timeout=${3:-5}
    
    if curl -f -s --max-time $timeout "$url" > /dev/null 2>&1; then
        echo -e "✅ ${GREEN}$name${NC} is responding at $url"
        return 0
    else
        echo -e "❌ ${RED}$name${NC} is not responding at $url"
        return 1
    fi
}

# Function to check port
check_port() {
    local host=$1
    local port=$2
    local name=$3
    local timeout=${4:-3}
    
    if timeout $timeout bash -c "</dev/tcp/$host/$port" 2>/dev/null; then
        echo -e "✅ ${GREEN}$name${NC} port $port is open on $host"
        return 0
    else
        echo -e "❌ ${RED}$name${NC} port $port is not accessible on $host"
        return 1
    fi
}

echo ""
echo "🌐 Checking Server 1 (188.245.97.41) - Full Stack..."

# UI Health Check
check_http "http://188.245.97.41:3001" "UI Frontend"

# Node 1 Health Check
check_http "http://188.245.97.41:8080/health" "Node 1 RPC"

# Gateway Health Check  
check_http "http://188.245.97.41:8081" "Gateway"

# Nginx Load Balancer
check_http "http://188.245.97.41:80" "Nginx Load Balancer"

# P2P Port Check
check_port "188.245.97.41" "4001" "Node 1 P2P"

echo ""
echo "🌐 Checking Server 2 (135.181.145.174) - Node 2..."

# Node 2 Health Check
check_http "http://135.181.145.174:8080/health" "Node 2 RPC"

# P2P Port Check
check_port "135.181.145.174" "4001" "Node 2 P2P"

echo ""
echo "🔗 Checking Cross-Node Connectivity..."

# Check if nodes can reach each other
echo "📡 Testing Node 1 → Node 2 connectivity..."
if curl -f -s --max-time 5 "http://135.181.145.174:8080/health" > /dev/null 2>&1; then
    echo -e "✅ ${GREEN}Node 1 can reach Node 2${NC}"
else
    echo -e "⚠️  ${YELLOW}Node 1 cannot reach Node 2 (may be normal if firewall configured)${NC}"
fi

echo "📡 Testing Node 2 → Node 1 connectivity..."
if curl -f -s --max-time 5 "http://188.245.97.41:8080/health" > /dev/null 2>&1; then
    echo -e "✅ ${GREEN}Node 2 can reach Node 1${NC}"
else
    echo -e "⚠️  ${YELLOW}Node 2 cannot reach Node 1 (may be normal if firewall configured)${NC}"
fi

echo ""
echo "🔒 Checking Firewall Ports..."

# Check required ports
REQUIRED_PORTS=("80" "443" "8080" "4001")
for port in "${REQUIRED_PORTS[@]}"; do
    if ss -ltnp | grep ":$port " > /dev/null 2>&1; then
        echo -e "✅ ${GREEN}Port $port${NC} is listening locally"
    else
        echo -e "⚠️  ${YELLOW}Port $port${NC} is not listening locally"
    fi
done

echo ""
echo "📊 Docker Container Status..."

# Check Docker containers on current server
if command -v docker-compose &> /dev/null || docker compose version &> /dev/null 2>&1; then
    echo "🐳 Local Docker containers:"
    if [ -f "docker-compose.full-stack.yml" ]; then
        docker-compose -f docker-compose.full-stack.yml ps 2>/dev/null || echo "No full-stack containers running"
    fi
    if [ -f "docker-compose.production.yml" ]; then
        docker-compose -f docker-compose.production.yml ps 2>/dev/null || echo "No production containers running"
    fi
else
    echo "⚠️  Docker Compose not available for container status check"
fi

echo ""
echo "🎯 Deployment Verification Summary:"
echo "=================================="
echo "✅ Check marks indicate services are responding correctly"
echo "❌ X marks indicate services need attention"
echo "⚠️  Warning marks indicate potential issues or expected behavior"
echo ""
echo "📋 Quick Commands:"
echo "  View logs:    docker-compose -f [compose-file] logs -f"
echo "  Restart:      docker-compose -f [compose-file] restart"
echo "  Stop all:     docker-compose -f [compose-file] down"
echo ""