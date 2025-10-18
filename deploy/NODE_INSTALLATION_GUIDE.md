# IPPAN Node Installation Guide

This guide provides step-by-step instructions for installing IPPAN blockchain nodes on both servers.

## 🎯 **Installation Options**

### **Option 1: Quick Installation (Recommended)**
Use the automated script for fastest deployment:

```bash
# Linux/macOS
./deploy/quick-install.sh

# Windows PowerShell
.\deploy\install-nodes.ps1
```

### **Option 2: Manual Installation**
Follow the detailed manual steps below.

### **Option 3: GitHub Actions**
Use the automated CI/CD pipeline:

```bash
# Trigger production deployment
gh workflow run prod-deploy.yml
```

---

## 🚀 **Quick Installation (Automated)**

### **Prerequisites**
- SSH access to both servers
- SSH keys configured
- `jq` and `curl` installed locally

### **Step 1: Run Quick Installation**

```bash
# Make scripts executable
chmod +x deploy/quick-install.sh

# Run quick installation
./deploy/quick-install.sh
```

### **Step 2: Verify Installation**

```bash
# Check Node 1
curl -sSL "http://188.245.97.41:8080/health" | jq '.'

# Check Node 2  
curl -sSL "http://135.181.145.174:8081/health" | jq '.'
```

---

## 🔧 **Manual Installation**

### **Server 1 (Primary: 188.245.97.41)**

#### **Step 1: Connect to Server**
```bash
ssh root@188.245.97.41
```

#### **Step 2: Update System**
```bash
apt-get update -y
DEBIAN_FRONTEND=noninteractive apt-get install -y \
    docker.io \
    docker-compose-plugin \
    jq \
    curl \
    ufw \
    htop
```

#### **Step 3: Start Docker**
```bash
systemctl start docker
systemctl enable docker
```

#### **Step 4: Create Node Directory**
```bash
mkdir -p /opt/ippan
cd /opt/ippan
```

#### **Step 5: Create Docker Compose Configuration**
```bash
cat > docker-compose.yml << 'EOF'
version: '3.8'

services:
  ippan-node:
    image: ghcr.io/dmrl789/ippan:latest
    container_name: ippan-node-1
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
      - NODE_ID=node-1
      - RPC_HOST=0.0.0.0
      - RPC_PORT=8080
      - P2P_HOST=0.0.0.0
      - P2P_PORT=9000
      - P2P_ANNOUNCE=/ip4/188.245.97.41/tcp/9000
      - DATA_DIR=/data
      - LOG_LEVEL=info
      - LOG_FORMAT=json
    ports:
      - "0.0.0.0:9000:9000"
      - "127.0.0.1:8080:8080"
    volumes:
      - ./data:/data
      - ./logs:/logs
    networks:
      - ippan-network
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

networks:
  ippan-network:
    driver: bridge
EOF
```

#### **Step 6: Configure Firewall**
```bash
ufw allow 9000/tcp comment "IPPAN P2P"
ufw allow 8080/tcp comment "IPPAN API"
ufw reload
```

#### **Step 7: Start Node**
```bash
docker compose up -d
```

#### **Step 8: Verify Node**
```bash
# Wait for node to start
sleep 30

# Check health
curl -sSL "http://127.0.0.1:8080/health" | jq '.'
```

---

### **Server 2 (Secondary: 135.181.145.174)**

#### **Step 1: Connect to Server**
```bash
ssh root@135.181.145.174
```

#### **Step 2-7: Repeat Steps 2-7 from Server 1**
Use the same commands but with these changes:
- Container name: `ippan-node-2`
- Node ID: `node-2`
- API Port: `8081`
- P2P Port: `9001`
- P2P Announce: `/ip4/135.181.145.174/tcp/9001`

#### **Step 8: Create Server 2 Configuration**
```bash
cat > docker-compose.yml << 'EOF'
version: '3.8'

services:
  ippan-node:
    image: ghcr.io/dmrl789/ippan:latest
    container_name: ippan-node-2
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
      - NODE_ID=node-2
      - RPC_HOST=0.0.0.0
      - RPC_PORT=8081
      - P2P_HOST=0.0.0.0
      - P2P_PORT=9001
      - P2P_ANNOUNCE=/ip4/135.181.145.174/tcp/9001
      - DATA_DIR=/data
      - LOG_LEVEL=info
      - LOG_FORMAT=json
    ports:
      - "0.0.0.0:9001:9001"
      - "127.0.0.1:8081:8081"
    volumes:
      - ./data:/data
      - ./logs:/logs
    networks:
      - ippan-network
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8081/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

networks:
  ippan-network:
    driver: bridge
EOF
```

---

## 🔗 **Configure P2P Connectivity**

### **Step 1: Add Bootstrap Peers**

On Server 1:
```bash
cd /opt/ippan
echo "IPPAN_P2P_BOOTSTRAP=/ip4/135.181.145.174/tcp/9001" >> .env
docker compose down
docker compose up -d
```

On Server 2:
```bash
cd /opt/ippan
echo "IPPAN_P2P_BOOTSTRAP=/ip4/188.245.97.41/tcp/9000" >> .env
docker compose down
docker compose up -d
```

### **Step 2: Verify Connectivity**
```bash
# Check both nodes
curl -sSL "http://188.245.97.41:8080/health" | jq '.'
curl -sSL "http://135.181.145.174:8081/health" | jq '.'
```

---

## 🏥 **Health Monitoring**

### **Check Node Status**
```bash
# Node 1
curl -sSL "http://188.245.97.41:8080/health"

# Node 2
curl -sSL "http://135.181.145.174:8081/health"
```

### **Check Docker Containers**
```bash
# On each server
docker ps
docker compose logs ippan-node
```

### **Check P2P Connectivity**
```bash
# Test P2P ports
telnet 188.245.97.41 9000
telnet 135.181.145.174 9001
```

---

## 🔧 **Troubleshooting**

### **Common Issues**

#### **Node Won't Start**
```bash
# Check logs
docker compose logs ippan-node

# Restart node
docker compose down
docker compose up -d
```

#### **Health Check Fails**
```bash
# Check if port is open
netstat -tlnp | grep :8080
netstat -tlnp | grep :8081

# Check firewall
ufw status
```

#### **P2P Connection Issues**
```bash
# Check P2P ports
netstat -tlnp | grep :9000
netstat -tlnp | grep :9001

# Test connectivity
telnet 188.245.97.41 9000
telnet 135.181.145.174 9001
```

### **Reset Installation**
```bash
# Stop and remove everything
docker compose down
docker system prune -f
rm -rf /opt/ippan/data
rm -rf /opt/ippan/logs

# Restart fresh
docker compose up -d
```

---

## 📊 **Verification Checklist**

- [ ] Node 1 is running on 188.245.97.41:8080
- [ ] Node 2 is running on 135.181.145.174:8081
- [ ] Both nodes respond to health checks
- [ ] P2P ports (9000, 9001) are open
- [ ] Docker containers are healthy
- [ ] Logs show no errors
- [ ] Nodes can communicate via P2P

---

## 🎉 **Installation Complete**

Once both nodes are running and healthy, you have successfully deployed the IPPAN blockchain network!

**Access Points:**
- **Node 1 API**: http://188.245.97.41:8080
- **Node 2 API**: http://135.181.145.174:8081
- **P2P Network**: Connected and operational

**Next Steps:**
- Monitor node health regularly
- Set up automated monitoring
- Configure backup procedures
- Plan for scaling additional nodes
