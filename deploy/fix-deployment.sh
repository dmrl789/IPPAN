#!/bin/bash
# IPPAN Deployment Fix Script
# This script fixes node connectivity and deploys unified UI

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
UI_DOMAIN="ui.ippan.org"

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

# Function to deploy unified UI
deploy_unified_ui() {
    echo -e "${BLUE}🌐 Deploying Unified UI to $SERVER1_IP...${NC}"
    
    # Create deployment directory structure
    cat > deploy-unified-ui.yml << 'EOF'
version: '3.8'

services:
  # IPPAN Blockchain Node
  ippan-node:
    image: ghcr.io/dmrl789/ippan/ippan-node:latest
    container_name: ippan-node
    environment:
      - NODE_ID=ippan_production_node_001
      - VALIDATOR_ID=0000000000000000000000000000000000000000000000000000000000000001
      - RPC_HOST=0.0.0.0
      - RPC_PORT=8080
      - P2P_HOST=0.0.0.0
      - P2P_PORT=9000
      - P2P_BOOTNODES=http://135.181.145.174:9001
      - STORAGE_PATH=/var/lib/ippan/db
      - LOG_LEVEL=info
      - LOG_FORMAT=json
    ports:
      - "8080:8080"  # RPC API
      - "9000:9000"  # P2P
    volumes:
      - node_data:/var/lib/ippan/db
    restart: unless-stopped
    networks:
      - ippan-network

  # Unified UI
  unified-ui:
    image: ghcr.io/dmrl789/ippan/unified-ui:latest
    container_name: unified-ui
    environment:
      - NODE_ENV=production
      - PORT=3000
      - NEXT_PUBLIC_GATEWAY_URL=https://ui.ippan.org/api
      - NEXT_PUBLIC_API_BASE_URL=https://ui.ippan.org/api
      - NEXT_PUBLIC_WS_URL=wss://ui.ippan.org/ws
      - NEXT_PUBLIC_ENABLE_FULL_UI=1
      - NEXT_PUBLIC_NETWORK_NAME=IPPAN-Devnet
    ports:
      - "3001:3000"  # UI
    restart: unless-stopped
    depends_on:
      - ippan-node
    networks:
      - ippan-network

  # API Gateway
  gateway:
    image: ghcr.io/dmrl789/ippan/gateway:latest
    container_name: gateway
    environment:
      - GATEWAY_HOST=0.0.0.0
      - GATEWAY_PORT=8081
      - NODE_RPC_URL=http://ippan-node:8080
      - GATEWAY_ALLOWED_ORIGINS=https://ui.ippan.org
    ports:
      - "8081:8081"  # Gateway API
    restart: unless-stopped
    depends_on:
      - ippan-node
    networks:
      - ippan-network

  # Nginx Reverse Proxy
  nginx:
    image: nginx:alpine
    container_name: nginx-proxy
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/nginx/ssl:ro
    restart: unless-stopped
    depends_on:
      - unified-ui
      - gateway
    networks:
      - ippan-network

volumes:
  node_data:

networks:
  ippan-network:
    driver: bridge
EOF

    # Create Nginx configuration
    cat > nginx.conf << 'EOF'
events {
    worker_connections 1024;
}

http {
    upstream ui {
        server unified-ui:3000;
    }
    
    upstream api {
        server gateway:8081;
    }
    
    server {
        listen 80;
        server_name ui.ippan.org;
        
        # Redirect HTTP to HTTPS
        return 301 https://$server_name$request_uri;
    }
    
    server {
        listen 443 ssl http2;
        server_name ui.ippan.org;
        
        # SSL configuration (you'll need to add your certificates)
        # ssl_certificate /etc/nginx/ssl/cert.pem;
        # ssl_certificate_key /etc/nginx/ssl/key.pem;
        
        # For now, disable SSL and use HTTP
        listen 80;
        
        location / {
            proxy_pass http://ui;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }
        
        location /api/ {
            proxy_pass http://api/;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }
        
        location /ws {
            proxy_pass http://api/ws;
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }
    }
}
EOF

    echo -e "${GREEN}✅ Deployment files created${NC}"
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
    
    # Deploy unified UI
    deploy_unified_ui
    
    # Create P2P fix script
    fix_p2p_connectivity
    
    echo -e "${GREEN}🎉 Deployment fix completed!${NC}"
    echo ""
    echo -e "${YELLOW}Next steps:${NC}"
    echo "1. Upload the deployment files to your server"
    echo "2. Run: docker-compose -f deploy-unified-ui.yml up -d"
    echo "3. Configure DNS to point ui.ippan.org to $SERVER1_IP"
    echo "4. Test the UI at http://$SERVER1_IP"
    echo ""
    echo -e "${BLUE}Files created:${NC}"
    echo "- deploy-unified-ui.yml (Docker Compose configuration)"
    echo "- nginx.conf (Nginx reverse proxy configuration)"
    echo "- fix-p2p.sh (P2P connectivity fix script)"
}

# Run main function
main "$@"
