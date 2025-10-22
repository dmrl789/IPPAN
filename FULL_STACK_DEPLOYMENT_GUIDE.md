# 🚀 IPPAN Full-Stack Deployment Guide

## 🌐 **Complete System Architecture**

Your IPPAN deployment includes:

- **Blockchain Nodes**: 2 IPPAN blockchain nodes for consensus
- **Load Balancer**: Nginx for distributing requests
- **API Gateway**: Unified API access across all nodes

---

## 📋 **Deployment Options**

### **Option 1: Full-Stack Docker Deployment (Recommended)**

#### **On Server 1 (188.245.97.41) - Primary Server:**

```bash
# Clone the repository
git clone <your-repo-url>
cd ippan

# Build and start all services
docker-compose -f deploy/docker-compose.full-stack.yml up -d

# Check status
docker-compose -f deploy/docker-compose.full-stack.yml ps
```

#### **On Server 2 (135.181.145.174) - Secondary Server:**

```bash
# Clone the repository
git clone <your-repo-url>
cd ippan

# Start the blockchain node
docker-compose -f deploy/docker-compose.production.yml up -d

# Check status
docker-compose -f deploy/docker-compose.production.yml ps
```

---

### **Option 2: Blockchain Nodes Only**

```bash
# Server 1
docker-compose -f deploy/docker-compose.production.yml up -d

# Server 2  
docker-compose -f deploy/docker-compose.production.yml up -d
```

---

## 🔧 **Service Configuration**

### **Service Ports**

| Service | Server 1 (188.245.97.41) | Server 2 (135.181.145.174) |
|---------|---------------------------|----------------------------|
| **UI** | 443 (HTTPS) | - |
| **Node 1 RPC** | 8080 | - |
| **Node 2 RPC** | - | 8080 |
| **Node 1 P2P** | 4001/tcp | - |
| **Node 2 P2P** | - | 4001/tcp |
| **Gateway / API** | 8080 (internal), 443 (public) | - |

### **Environment Variables**

**UI Configuration:**
```bash
REACT_APP_API_URL=https://api.ippan.org
REACT_APP_NODE_1_URL=https://api.ippan.org
REACT_APP_NODE_2_URL=https://api.ippan.org
REACT_APP_WS_URL=wss://api.ippan.org/ws
REACT_APP_ENABLE_FULL_UI=1
```

---

## 🌐 **Access Points**

### **Web Interface**
- **Primary UI**: https://ui.ippan.org
- **Gateway**: https://api.ippan.org

### **API Endpoints**
- **Gateway API**: https://api.ippan.org

### **Health Checks**
- **UI Health**: https://ui.ippan.org/health
- **Gateway Health**: https://api.ippan.org/health

---

## 🚀 **Quick Start Commands**

### **Full Deployment (Server 1)**
```bash
# Clone and deploy everything
git clone <your-repo-url>
cd ippan
docker-compose -f deploy/docker-compose.full-stack.yml up -d

# Check all services
docker-compose -f deploy/docker-compose.full-stack.yml ps

# View logs
docker-compose -f deploy/docker-compose.full-stack.yml logs -f
```

### **Blockchain Only (Server 2)**
```bash
# Clone and deploy blockchain node
git clone <your-repo-url>
cd ippan
docker-compose -f deploy/docker-compose.production.yml up -d

# Check node status
docker-compose -f deploy/docker-compose.production.yml ps
```

---

## 🔍 **Verification Steps**

### **1. Check All Services**

```bash
# Check UI
curl https://ui.ippan.org/health

# Check Node 1
curl https://api.ippan.org/health

# Check Node 2
curl https://api.ippan.org/health

# Check WebSocket upgrade
curl -I -H 'Connection: Upgrade' -H 'Upgrade: websocket' https://ui.ippan.org/ws
```

### **2. Test UI Functionality**

```bash
# Open web browser and navigate to:
# https://ui.ippan.org

# Test API through UI
curl https://api.ippan.org/health
```

### **3. Verify Network Connectivity**

```bash
# Confirm each node listens publicly for libp2p traffic
ss -ltnp | grep :4001 || sudo lsof -iTCP:4001 -sTCP:LISTEN

# Open the firewall if required
sudo ufw allow 4001/tcp
sudo ufw reload

# Check peer connections from the HTTPS gateway
curl https://api.ippan.org/peers

# Test transaction propagation through the HTTPS gateway
curl -X POST https://api.ippan.org/tx \
  -H "Content-Type: application/json" \
  -d '{
    "from": "0000000000000000000000000000000000000000000000000000000000000001",
    "to": "0000000000000000000000000000000000000000000000000000000000000002",
    "amount": 1000,
    "nonce": 1,
    "signature": "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
  }'
```


---

## 📊 **Monitoring & Logs**

### **View Logs**

```bash
# All services
docker-compose -f deploy/docker-compose.full-stack.yml logs -f

# Specific service
docker-compose -f deploy/docker-compose.full-stack.yml logs -f ippan-ui
docker-compose -f deploy/docker-compose.full-stack.yml logs -f ippan-node-1
```

### **Service Status**

```bash
# Check running containers
docker ps

# Check service health
docker-compose -f deploy/docker-compose.full-stack.yml ps
```

### **Resource Usage**

```bash
# Check resource usage
docker stats

# Check disk usage
docker system df
```

---

## 🔧 **Configuration Management**

### **UI Configuration**

The UI can be configured via environment variables:

```bash
# In docker-compose.full-stack.yml
environment:
  - REACT_APP_API_URL=https://api.ippan.org
  - REACT_APP_NODE_1_URL=https://api.ippan.org
  - REACT_APP_NODE_2_URL=https://api.ippan.org
  - REACT_APP_WS_URL=wss://api.ippan.org/ws
  - REACT_APP_ENABLE_FULL_UI=1
  - REACT_APP_NETWORK_NAME=IPPAN Production
  - REACT_APP_CHAIN_ID=ippan-mainnet
```

### **Load Balancer Configuration**

Edit `deploy/nginx/load-balancer.conf` to modify:
- Upstream servers
- Load balancing algorithm
- Health check intervals
- SSL/TLS configuration

---

## 🚨 **Troubleshooting**

### **Common Issues**

**1. UI not loading:**
```bash
# Check UI container
docker logs ippan-ui

# Check nginx configuration
docker exec ippan-ui nginx -t
```

**2. API not accessible:**
```bash
# Check load balancer
docker logs ippan-nginx-lb

# Test direct node access through the gateway
curl https://api.ippan.org/health
```

**3. Nodes not connecting:**
```bash
# Check P2P connectivity
docker logs ippan-node-1
docker logs ippan-node-2

# Test network connectivity
ss -ltnp | grep :4001 || sudo lsof -iTCP:4001 -sTCP:LISTEN
sudo ufw status | grep 4001 || sudo ufw allow 4001/tcp
```

### **Debug Commands**

```bash
# Check all service logs
docker-compose -f deploy/docker-compose.full-stack.yml logs

# Restart specific service
docker-compose -f deploy/docker-compose.full-stack.yml restart ippan-ui

# Rebuild and restart
docker-compose -f deploy/docker-compose.full-stack.yml up -d --build
```

---

## 🔒 **Security Considerations**

### **Firewall Configuration**

```bash
# On both servers
sudo ufw allow 80/tcp    # HTTP (redirect to HTTPS)
sudo ufw allow 443/tcp   # Public HTTPS entrypoint
sudo ufw allow 8080/tcp  # RPC API (internal)
sudo ufw allow 4001/tcp  # P2P libp2p port
```

### **SSL/TLS Setup (Optional)**

```bash
# Generate SSL certificates
sudo certbot --nginx -d your-domain.com

# Update nginx configuration for HTTPS
# Edit deploy/nginx/load-balancer.conf
```

---

## 📈 **Scaling Options**

### **Horizontal Scaling**

1. **Add more blockchain nodes**
2. **Deploy UI on multiple servers**
3. **Use CDN for static assets**
4. **Implement database clustering**

### **Vertical Scaling**

1. **Increase container resources**
2. **Optimize nginx configuration**
3. **Enable caching layers**
4. **Implement connection pooling**

---

## 🎯 **Next Steps**

1. **Deploy the full stack** using Docker Compose
2. **Access the web UI** at https://ui.ippan.org
3. **Test blockchain functionality** through the UI
4. **Monitor system performance** and logs
5. **Set up automated backups** for blockchain data
6. **Configure monitoring and alerting**

---

## 📝 **Summary**

✅ **Complete IPPAN deployment** with UI and blockchain nodes  
✅ **Load balancing** for high availability  
✅ **Production-ready** configuration  
✅ **Easy scaling** and maintenance  
✅ **Comprehensive monitoring** and logging  

**Your IPPAN blockchain network with unified UI is ready for production!** 🚀

### **Access Your System:**
- **Web UI**: https://ui.ippan.org
- **API**: https://api.ippan.org
