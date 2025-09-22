# 🚀 IPPAN Production Deployment Guide

## 🌐 **Your Server Configuration**

- **Server 1**: `188.245.97.41` (Node 1)
- **Server 2**: `135.181.145.174` (Node 2)

---

## 📋 **Deployment Options**

### **Option 1: Docker Deployment (Recommended)**

#### **On Server 1 (188.245.97.41):**

```bash
# Clone the repository
git clone <your-repo-url>
cd ippan

# Build the Docker image
docker build -f Dockerfile.production -t ippan-node:latest .

# Start Node 1
docker run -d \
  --name ippan-node-1 \
  --restart unless-stopped \
  -p 8080:8080 \
  -p 9000:9000 \
  -v ippan_data:/var/lib/ippan/db \
  -e NODE_ID=ippan_production_node_001 \
  -e VALIDATOR_ID=0000000000000000000000000000000000000000000000000000000000000001 \
  -e RPC_HOST=0.0.0.0 \
  -e RPC_PORT=8080 \
  -e P2P_HOST=0.0.0.0 \
  -e P2P_PORT=9000 \
  -e P2P_BOOTNODES=http://135.181.145.174:9001 \
  -e STORAGE_PATH=/var/lib/ippan/db \
  -e LOG_LEVEL=info \
  -e LOG_FORMAT=json \
  ippan-node:latest
```

#### **On Server 2 (135.181.145.174):**

```bash
# Clone the repository
git clone <your-repo-url>
cd ippan

# Build the Docker image
docker build -f Dockerfile.production -t ippan-node:latest .

# Start Node 2
docker run -d \
  --name ippan-node-2 \
  --restart unless-stopped \
  -p 8080:8080 \
  -p 9001:9001 \
  -v ippan_data:/var/lib/ippan/db \
  -e NODE_ID=ippan_production_node_002 \
  -e VALIDATOR_ID=0000000000000000000000000000000000000000000000000000000000000002 \
  -e RPC_HOST=0.0.0.0 \
  -e RPC_PORT=8080 \
  -e P2P_HOST=0.0.0.0 \
  -e P2P_PORT=9001 \
  -e P2P_BOOTNODES=http://188.245.97.41:9000 \
  -e STORAGE_PATH=/var/lib/ippan/db \
  -e LOG_LEVEL=info \
  -e LOG_FORMAT=json \
  ippan-node:latest
```

---

### **Option 2: Native Binary Deployment**

#### **On Server 1 (188.245.97.41):**

```bash
# Clone and build
git clone <your-repo-url>
cd ippan
cargo build --release --workspace

# Create systemd service
sudo cp deploy/ippan-node.service /etc/systemd/system/

# Create configuration directory
sudo mkdir -p /etc/ippan

# Copy environment configuration
sudo cp config/production-node1.env /etc/ippan/ippan.env

# Create data directory
sudo mkdir -p /var/lib/ippan/db
sudo chown -R ippan:ippan /var/lib/ippan

# Start the service
sudo systemctl daemon-reload
sudo systemctl enable ippan-node
sudo systemctl start ippan-node
```

#### **On Server 2 (135.181.145.174):**

```bash
# Clone and build
git clone <your-repo-url>
cd ippan
cargo build --release --workspace

# Create systemd service
sudo cp deploy/ippan-node.service /etc/systemd/system/

# Create configuration directory
sudo mkdir -p /etc/ippan

# Copy environment configuration
sudo cp config/production-node2.env /etc/ippan/ippan.env

# Create data directory
sudo mkdir -p /var/lib/ippan/db
sudo chown -R ippan:ippan /var/lib/ippan

# Start the service
sudo systemctl daemon-reload
sudo systemctl enable ippan-node
sudo systemctl start ippan-node
```

---

## 🔍 **Verification Steps**

### **1. Check Node Status**

**Server 1 (188.245.97.41):**
```bash
# Check if node is running
curl http://188.245.97.41:8080/health

# Check peer connections
curl http://188.245.97.41:8080/p2p/peers

# Check node status
curl http://188.245.97.41:8080/status
```

**Server 2 (135.181.145.174):**
```bash
# Check if node is running
curl http://135.181.145.174:8080/health

# Check peer connections
curl http://135.181.145.174:8080/p2p/peers

# Check node status
curl http://135.181.145.174:8080/status
```

### **2. Test Network Connectivity**

```bash
# From Server 1, test connection to Server 2
curl http://135.181.145.174:8080/health

# From Server 2, test connection to Server 1
curl http://188.245.97.41:8080/health
```

### **3. Test Transaction Propagation**

```bash
# Submit a transaction to Server 1
curl -X POST http://188.245.97.41:8080/tx \
  -H "Content-Type: application/json" \
  -d '{
    "from": "0000000000000000000000000000000000000000000000000000000000000001",
    "to": "0000000000000000000000000000000000000000000000000000000000000002",
    "amount": 1000,
    "nonce": 1,
    "signature": "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
  }'

# Check if transaction appears on Server 2
curl http://135.181.145.174:8080/accounts
```

---

## 🔧 **Configuration Details**

### **Network Configuration**

| Server | IP Address | RPC Port | P2P Port | Bootstrap Nodes |
|--------|------------|----------|----------|-----------------|
| Node 1 | 188.245.97.41 | 8080 | 9000 | http://135.181.145.174:9001 |
| Node 2 | 135.181.145.174 | 8080 | 9001 | http://188.245.97.41:9000 |

### **Validator Configuration**

- **Node 1**: Validator ID `0000000000000000000000000000000000000000000000000000000000000001`
- **Node 2**: Validator ID `0000000000000000000000000000000000000000000000000000000000000002`

### **Firewall Configuration**

Make sure these ports are open on both servers:

```bash
# On both servers
sudo ufw allow 8080/tcp  # RPC API
sudo ufw allow 9000/tcp  # P2P (Node 1)
sudo ufw allow 9001/tcp  # P2P (Node 2)
```

---

## 📊 **Monitoring**

### **Check Logs**

**Docker:**
```bash
# Server 1
docker logs -f ippan-node-1

# Server 2
docker logs -f ippan-node-2
```

**Systemd:**
```bash
# Both servers
sudo journalctl -u ippan-node -f
```

### **Health Monitoring**

```bash
# Create a monitoring script
cat > monitor.sh << 'EOF'
#!/bin/bash

echo "=== IPPAN Network Status ==="
echo "Server 1 (188.245.97.41):"
curl -s http://188.245.97.41:8080/health | jq

echo "Server 2 (135.181.145.174):"
curl -s http://135.181.145.174:8080/health | jq

echo "=== Peer Connections ==="
echo "Server 1 peers:"
curl -s http://188.245.97.41:8080/p2p/peers | jq

echo "Server 2 peers:"
curl -s http://135.181.145.174:8080/p2p/peers | jq
EOF

chmod +x monitor.sh
./monitor.sh
```

---

## 🚨 **Troubleshooting**

### **Common Issues**

**1. Nodes can't connect to each other:**
```bash
# Check firewall
sudo ufw status

# Test network connectivity
telnet 188.245.97.41 9000
telnet 135.181.145.174 9001
```

**2. P2P_BOOTNODES not working:**
```bash
# Check environment variables
docker exec ippan-node-1 env | grep P2P_BOOTNODES
docker exec ippan-node-2 env | grep P2P_BOOTNODES
```

**3. Transactions not propagating:**
```bash
# Check peer connections
curl http://188.245.97.41:8080/p2p/peers
curl http://135.181.145.174:8080/p2p/peers

# Check transaction storage
curl http://188.245.97.41:8080/accounts
curl http://135.181.145.174:8080/accounts
```

---

## 🎯 **Next Steps**

1. **Deploy both nodes** using your preferred method (Docker or native)
2. **Verify network connectivity** between the servers
3. **Test transaction propagation** across the network
4. **Monitor block production** and synchronization
5. **Set up monitoring** and alerting for production use

---

## 📝 **Notes**

- **No hardcoding required** - All IPs are configured via environment variables
- **Easy to scale** - Add more nodes by updating P2P_BOOTNODES
- **Production ready** - Includes logging, monitoring, and error handling
- **Flexible deployment** - Docker or native binary options available

**Your IPPAN blockchain network is ready for production deployment!** 🚀
