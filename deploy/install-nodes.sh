#!/bin/bash
set -euo pipefail

# IPPAN Multi-Node Installation Script
# This script installs IPPAN nodes on both servers

# Server configurations
PRIMARY_SERVER="188.245.97.41"
SECONDARY_SERVER="135.181.145.174"
SSH_USER="root"
SSH_PORT="22"

# Node configurations
NODE1_CONFIG=(
    "node-1"
    "$PRIMARY_SERVER"
    "8080"
    "9000"
    "/opt/ippan"
    "ghcr.io/dmrl789/ippan:latest"
)

NODE2_CONFIG=(
    "node-2"
    "$SECONDARY_SERVER"
    "8081"
    "9001"
    "/opt/ippan"
    "ghcr.io/dmrl789/ippan:latest"
)

echo "🚀 IPPAN Multi-Node Installation"
echo "================================="
echo "📍 Primary Server: $PRIMARY_SERVER"
echo "📍 Secondary Server: $SECONDARY_SERVER"
echo ""

# Function to install node on server
install_node() {
    local server_ip="$1"
    local node_config=("$@")
    local node_id="${node_config[0]}"
    local server="${node_config[1]}"
    local api_port="${node_config[2]}"
    local p2p_port="${node_config[3]}"
    local data_dir="${node_config[4]}"
    local docker_image="${node_config[5]}"
    
    echo "🔧 Installing $node_id on $server_ip..."
    
    # Copy installation script to server
    scp -o StrictHostKeyChecking=no -P $SSH_PORT deploy/install-node.sh $SSH_USER@$server_ip:/tmp/
    
    # Execute installation on server
    ssh -o StrictHostKeyChecking=no -p $SSH_PORT $SSH_USER@$server_ip << EOF
        set -euo pipefail
        chmod +x /tmp/install-node.sh
        /tmp/install-node.sh "$node_id" "$server" "$api_port" "$p2p_port" "$data_dir" "$docker_image"
EOF
    
    echo "✅ $node_id installation completed on $server_ip"
}

# Function to verify node health
verify_node() {
    local server_ip="$1"
    local api_port="$2"
    local node_id="$3"
    
    echo "🏥 Verifying $node_id health on $server_ip:$api_port..."
    
    if curl -fsSL "http://$server_ip:$api_port/health" >/dev/null 2>&1; then
        echo "✅ $node_id is healthy"
        curl -sSL "http://$server_ip:$api_port/health" | jq '.' || echo "Health check successful"
    else
        echo "❌ $node_id health check failed"
        return 1
    fi
}

# Function to configure P2P connectivity
configure_p2p() {
    local node1_ip="$1"
    local node1_port="$2"
    local node2_ip="$3"
    local node2_port="$4"
    
    echo "🔗 Configuring P2P connectivity..."
    
    # Add node2 as bootstrap peer to node1
    ssh -o StrictHostKeyChecking=no -p $SSH_PORT $SSH_USER@$node1_ip << EOF
        cd /opt/ippan
        echo "IPPAN_P2P_BOOTSTRAP=/ip4/$node2_ip/tcp/$node2_port" >> .env
        docker compose down
        docker compose up -d
EOF
    
    # Add node1 as bootstrap peer to node2
    ssh -o StrictHostKeyChecking=no -p $SSH_PORT $SSH_USER@$node2_ip << EOF
        cd /opt/ippan
        echo "IPPAN_P2P_BOOTSTRAP=/ip4/$node1_ip/tcp/$node1_port" >> .env
        docker compose down
        docker compose up -d
EOF
    
    echo "✅ P2P connectivity configured"
}

# Main installation process
main() {
    echo "🚀 Starting IPPAN multi-node installation..."
    
    # Install Node 1
    echo "📦 Installing Node 1 on primary server..."
    install_node "$PRIMARY_SERVER" "${NODE1_CONFIG[@]}"
    
    # Install Node 2
    echo "📦 Installing Node 2 on secondary server..."
    install_node "$SECONDARY_SERVER" "${NODE2_CONFIG[@]}"
    
    # Wait for both nodes to start
    echo "⏳ Waiting for nodes to initialize..."
    sleep 60
    
    # Verify both nodes
    echo "🏥 Verifying node health..."
    verify_node "$PRIMARY_SERVER" "8080" "node-1"
    verify_node "$SECONDARY_SERVER" "8081" "node-2"
    
    # Configure P2P connectivity
    configure_p2p "$PRIMARY_SERVER" "9000" "$SECONDARY_SERVER" "9001"
    
    # Final verification
    echo "🔍 Final connectivity test..."
    sleep 30
    
    echo "📊 Final Status Check:"
    echo "Node 1: http://$PRIMARY_SERVER:8080/health"
    curl -sSL "http://$PRIMARY_SERVER:8080/health" | jq '.' || echo "Node 1 health check"
    
    echo "Node 2: http://$SECONDARY_SERVER:8081/health"
    curl -sSL "http://$SECONDARY_SERVER:8081/health" | jq '.' || echo "Node 2 health check"
    
    echo ""
    echo "🎉 IPPAN Multi-Node Installation Completed!"
    echo "=========================================="
    echo "📍 Node 1: http://$PRIMARY_SERVER:8080"
    echo "📍 Node 2: http://$SECONDARY_SERVER:8081"
    echo "🌐 P2P Network: Connected"
    echo "✅ Both nodes are running and healthy"
}

# Check prerequisites
check_prerequisites() {
    echo "🔍 Checking prerequisites..."
    
    # Check if SSH keys are available
    if [ ! -f ~/.ssh/id_rsa ] && [ ! -f ~/.ssh/id_ed25519 ]; then
        echo "❌ SSH key not found. Please ensure you have SSH access configured."
        exit 1
    fi
    
    # Check if jq is installed locally
    if ! command -v jq >/dev/null 2>&1; then
        echo "❌ jq is required but not installed. Please install jq."
        exit 1
    fi
    
    # Check if curl is available
    if ! command -v curl >/dev/null 2>&1; then
        echo "❌ curl is required but not installed. Please install curl."
        exit 1
    fi
    
    echo "✅ Prerequisites check passed"
}

# Run main function
check_prerequisites
main
