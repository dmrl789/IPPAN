#!/bin/bash
# IPPAN Hetzner Devnet Node Setup Script
# Run this script on each Hetzner server after provisioning

set -euo pipefail

NODE_ID="${1:-}"
NODE1_IP="${2:-}"

if [ -z "$NODE_ID" ]; then
    echo "Usage: $0 <node_id> [node1_ip]"
    echo "Example: $0 node1"
    echo "Example: $0 node2 10.0.0.1"
    exit 1
fi

echo "=========================================="
echo "IPPAN Devnet Node Setup: $NODE_ID"
echo "=========================================="

# STEP 1: Update system
echo "[1/8] Updating system packages..."
sudo apt update && sudo apt -y upgrade

# STEP 2: Install dependencies
echo "[2/8] Installing system dependencies..."
sudo apt -y install build-essential pkg-config libssl-dev clang cmake git ufw jq curl

# STEP 3: Install Rust
echo "[3/8] Installing Rust toolchain..."
if ! command -v rustc &> /dev/null; then
    curl https://sh.rustup.rs -sSf | sh -s -- -y
    source $HOME/.cargo/env
    rustup default stable
else
    echo "Rust already installed: $(rustc --version)"
fi

# STEP 4: Clone repo and build
echo "[4/8] Cloning repository and building binary..."
sudo mkdir -p /opt/ippan
sudo chown $USER:$USER /opt/ippan
cd /opt/ippan

if [ ! -d ".git" ]; then
    git clone https://github.com/dmrl789/IPPAN .
    git checkout master
else
    echo "Repository already exists, pulling latest..."
    git pull origin master
fi

echo "Building release binary (this may take 15-30 minutes)..."
cargo build --release -p ippan-node

# STEP 5: Create directories
echo "[5/8] Creating configuration and data directories..."
sudo mkdir -p /etc/ippan /var/lib/ippan /var/log/ippan
sudo chown $USER:$USER /etc/ippan /var/lib/ippan /var/log/ippan

# Copy config
cp -r config /etc/ippan/config

# STEP 6: Create node-specific config
echo "[6/8] Creating node configuration..."
BOOTSTRAP_NODES=""
if [ "$NODE_ID" != "node1" ] && [ -n "$NODE1_IP" ]; then
    BOOTSTRAP_NODES="http://${NODE1_IP}:9000"
fi

cat > /etc/ippan/config/node.toml << EOF
[node]
id = "ippan-devnet-${NODE_ID}"

[network]
id = "ippan-devnet"

[rpc]
host = "0.0.0.0"
port = 8080

[p2p]
host = "0.0.0.0"
port = 9000
bootstrap_nodes = "${BOOTSTRAP_NODES}"

[storage]
data_dir = "/var/lib/ippan"
db_path = "/var/lib/ippan/db"

[logging]
level = "info"
format = "json"

[metrics]
enabled = true
EOF

echo "Configuration written to /etc/ippan/config/node.toml"

# STEP 7: Setup systemd service
echo "[7/8] Setting up systemd service..."
sudo tee /etc/systemd/system/ippan-node.service > /dev/null << 'EOF'
[Unit]
Description=IPPAN Node
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/ippan
Environment=RUST_LOG=info
Environment=RUST_BACKTRACE=1
ExecStart=/opt/ippan/target/release/ippan-node --config /etc/ippan/config/node.toml
Restart=always
RestartSec=2
LimitNOFILE=1048576
StandardOutput=append:/var/log/ippan/node.log
StandardError=append:/var/log/ippan/node.err

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable ippan-node

# STEP 8: Configure firewall
echo "[8/8] Configuring firewall..."
sudo ufw allow 22/tcp
sudo ufw allow 8080/tcp
sudo ufw allow 9000/tcp
sudo ufw allow 9000/udp
sudo ufw --force enable

echo "=========================================="
echo "Setup complete!"
echo "=========================================="
echo ""
echo "To start the node, run:"
echo "  sudo systemctl start ippan-node"
echo ""
echo "To check status:"
echo "  sudo systemctl status ippan-node"
echo ""
echo "To view logs:"
echo "  sudo journalctl -u ippan-node -f"
echo "  tail -f /var/log/ippan/node.log"
echo ""

