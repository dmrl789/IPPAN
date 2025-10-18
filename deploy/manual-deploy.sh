#!/bin/bash
set -euo pipefail

# Manual IPPAN Node Deployment Script
# This script provides step-by-step commands for manual deployment

echo "🚀 Manual IPPAN Node Deployment"
echo "================================"
echo ""
echo "This script will guide you through manual deployment of IPPAN nodes."
echo "You can run these commands on each server or use SSH to execute them remotely."
echo ""

# Server configurations
PRIMARY_SERVER="188.245.97.41"
SECONDARY_SERVER="135.181.145.174"

echo "📍 Primary Server: $PRIMARY_SERVER"
echo "📍 Secondary Server: $SECONDARY_SERVER"
echo ""

# Function to generate deployment commands
generate_deployment_commands() {
    local server_ip="$1"
    local node_id="$2"
    local api_port="$3"
    local p2p_port="$4"
    
    echo "🔧 Deployment Commands for $node_id on $server_ip:"
    echo "=================================================="
    echo ""
    echo "# 1. Connect to server"
    echo "ssh root@$server_ip"
    echo ""
    echo "# 2. Update system and install packages"
    echo "apt-get update -y"
    echo "DEBIAN_FRONTEND=noninteractive apt-get install -y docker.io docker-compose-plugin jq curl ufw htop"
    echo ""
    echo "# 3. Start Docker"
    echo "systemctl start docker"
    echo "systemctl enable docker"
    echo ""
    echo "# 4. Create node directory"
    echo "mkdir -p /opt/ippan"
    echo "cd /opt/ippan"
    echo ""
    echo "# 5. Create Docker Compose configuration"
    cat << EOF
cat > docker-compose.yml << 'YML'
version: '3.8'

services:
  ippan-node:
    image: ghcr.io/dmrl789/ippan:latest
    container_name: ippan-$node_id
    restart: unless-stopped
    user: "0:0"
    command:
      - sh
      - -lc
      - |
        set -e
        echo 'deb http://deb.debian.org/debian bookworm main' > /etc/apt/sources.list.d/bookworm.list
        apt-get update -y
        apt-get install -y --no-install-recommends -t bookworm libssl3 ca-certificates
        exec ippan-node
    environment:
      - NODE_ID=$node_id
      - RPC_HOST=0.0.0.0
      - RPC_PORT=$api_port
      - P2P_HOST=0.0.0.0
      - P2P_PORT=$p2p_port
      - P2P_ANNOUNCE=/ip4/$server_ip/tcp/$p2p_port
      - DATA_DIR=/data
      - LOG_LEVEL=info
      - LOG_FORMAT=json
    ports:
      - "0.0.0.0:$p2p_port:$p2p_port"
      - "127.0.0.1:$api_port:$api_port"
    volumes:
      - ./data:/data
      - ./logs:/logs
    networks:
      - ippan-network
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:$api_port/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

networks:
  ippan-network:
    driver: bridge
YML
EOF
    echo ""
    echo "# 6. Configure firewall"
    echo "ufw allow $p2p_port/tcp comment \"IPPAN P2P\""
    echo "ufw allow $api_port/tcp comment \"IPPAN API\""
    echo "ufw reload"
    echo ""
    echo "# 7. Start the node"
    echo "docker compose up -d"
    echo ""
    echo "# 8. Wait for node to start"
    echo "sleep 30"
    echo ""
    echo "# 9. Verify node health"
    echo "curl -sSL \"http://127.0.0.1:$api_port/health\" | jq '.'"
    echo ""
    echo "# 10. Check container status"
    echo "docker ps"
    echo "docker compose logs ippan-$node_id"
    echo ""
}

# Generate commands for both nodes
echo "📋 NODE 1 DEPLOYMENT COMMANDS"
echo "=============================="
generate_deployment_commands "$PRIMARY_SERVER" "node-1" "8080" "9000"

echo ""
echo "📋 NODE 2 DEPLOYMENT COMMANDS"
echo "=============================="
generate_deployment_commands "$SECONDARY_SERVER" "node-2" "8081" "9001"

echo ""
echo "🔗 P2P CONNECTIVITY SETUP"
echo "=========================="
echo ""
echo "# After both nodes are running, configure P2P connectivity:"
echo ""
echo "# On Node 1 (Primary Server):"
echo "ssh root@$PRIMARY_SERVER"
echo "cd /opt/ippan"
echo "echo \"IPPAN_P2P_BOOTSTRAP=/ip4/$SECONDARY_SERVER/tcp/9001\" >> .env"
echo "docker compose down"
echo "docker compose up -d"
echo ""
echo "# On Node 2 (Secondary Server):"
echo "ssh root@$SECONDARY_SERVER"
echo "cd /opt/ippan"
echo "echo \"IPPAN_P2P_BOOTSTRAP=/ip4/$PRIMARY_SERVER/tcp/9000\" >> .env"
echo "docker compose down"
echo "docker compose up -d"
echo ""

echo "🏥 VERIFICATION COMMANDS"
echo "========================"
echo ""
echo "# Check Node 1 health:"
echo "curl -sSL \"http://$PRIMARY_SERVER:8080/health\" | jq '.'"
echo ""
echo "# Check Node 2 health:"
echo "curl -sSL \"http://$SECONDARY_SERVER:8081/health\" | jq '.'"
echo ""
echo "# Test P2P connectivity:"
echo "telnet $PRIMARY_SERVER 9000"
echo "telnet $SECONDARY_SERVER 9001"
echo ""

echo "🎉 DEPLOYMENT COMPLETE"
echo "======================"
echo ""
echo "Once both nodes are running and healthy:"
echo "📍 Node 1 API: http://$PRIMARY_SERVER:8080"
echo "📍 Node 2 API: http://$SECONDARY_SERVER:8081"
echo "🌐 P2P Network: Connected and operational"
echo ""
echo "For troubleshooting, check:"
echo "- Docker logs: docker compose logs ippan-node"
echo "- Container status: docker ps"
echo "- Health endpoints: curl http://server:port/health"
echo "- P2P ports: netstat -tlnp | grep :900"
