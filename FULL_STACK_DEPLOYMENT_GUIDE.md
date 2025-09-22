# 🚀 IPPAN Full-Stack Deployment Guide

## 🌐 **Complete System Architecture**

Your IPPAN deployment includes:

- **Blockchain Nodes**: 2 IPPAN blockchain nodes for consensus
- **Unified UI**: React-based web interface for blockchain interaction
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

# Start only the blockchain node (UI runs on Server 1)
docker-compose -f deploy/docker-compose.production.yml up -d

# Check status
docker-compose -f deploy/docker-compose.production.yml ps
```

---

### **Option 2: Separate UI Deployment**

#### **UI on Server 1 (188.245.97.41):**

```bash
# Build and run UI
cd apps/unified-ui
docker build -t ippan-ui:latest .
docker run -d \
  --name ippan-ui \
  --restart unless-stopped \
  -p 80:80 \
  -e REACT_APP_API_URL=http://188.245.97.41:8080 \
  -e REACT_APP_NODE_1_URL=http://188.245.97.41:8080 \
  -e REACT_APP_NODE_2_URL=http://135.181.145.174:8080 \
  ippan-ui:latest
```

#### **Blockchain Nodes on Both Servers:**

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
| **UI** | 80 (HTTP) | - |
| **Node 1 RPC** | 8080 | - |
| **Node 2 RPC** | - | 8080 |
| **Node 1 P2P** | 9000 | - |
| **Node 2 P2P** | - | 9001 |
| **Load Balancer** | 3000 | - |

### **Environment Variables**

**UI Configuration:**
```bash
REACT_APP_API_URL=http://188.245.97.41:8080
REACT_APP_NODE_1_URL=http://188.245.97.41:8080
REACT_APP_NODE_2_URL=http://135.181.145.174:8080
```

---

## 🌐 **Access Points**

### **Web Interface**
- **Primary UI**: http://188.245.97.41
- **Load Balancer**: http://188.245.97.41:3000

### **API Endpoints**
- **Node 1 API**: http://188.245.97.41:8080
- **Node 2 API**: http://135.181.145.174:8080
- **Load Balanced API**: http://188.245.97.41:3000/api

### **Health Checks**
- **UI Health**: http://188.245.97.41/health
- **Node 1 Health**: http://188.245.97.41:8080/health
- **Node 2 Health**: http://135.181.145.174:8080/health
- **Load Balancer Health**: http://188.245.97.41:3000/lb-health

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
curl http://188.245.97.41/health

# Check Node 1
curl http://188.245.97.41:8080/health

# Check Node 2
curl http://135.181.145.174:8080/health

# Check Load Balancer
curl http://188.245.97.41:3000/lb-health
```

### **2. Test UI Functionality**

```bash
# Open web browser and navigate to:
# http://188.245.97.41

# Test API through UI
curl http://188.245.97.41:3000/api/health
```

### **3. Verify Network Connectivity**

```bash
# Check peer connections
curl http://188.245.97.41:8080/p2p/peers
curl http://135.181.145.174:8080/p2p/peers

# Test transaction propagation
curl -X POST http://188.245.97.41:8080/tx \
  -H "Content-Type: application/json" \
  -d '{
    "from": "0000000000000000000000000000000000000000000000000000000000000001",
    "to": "0000000000000000000000000000000000000000000000000000000000000002",
    "amount": 1000,
    "nonce": 1,
    "signature": "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
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
  - REACT_APP_API_URL=http://188.245.97.41:8080
  - REACT_APP_NODE_1_URL=http://188.245.97.41:8080
  - REACT_APP_NODE_2_URL=http://135.181.145.174:8080
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

# Test direct node access
curl http://188.245.97.41:8080/health
```

**3. Nodes not connecting:**
```bash
# Check P2P connectivity
docker logs ippan-node-1
docker logs ippan-node-2

# Test network connectivity
telnet 188.245.97.41 9000
telnet 135.181.145.174 9001
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
sudo ufw allow 80/tcp    # HTTP
sudo ufw allow 443/tcp   # HTTPS
sudo ufw allow 8080/tcp  # RPC API
sudo ufw allow 9000/tcp  # P2P Node 1
sudo ufw allow 9001/tcp  # P2P Node 2
sudo ufw allow 3000/tcp  # Load Balancer
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
2. **Access the web UI** at http://188.245.97.41
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
- **Web UI**: http://188.245.97.41
- **API**: http://188.245.97.41:3000/api
- **Node 1**: http://188.245.97.41:8080
- **Node 2**: http://135.181.145.174:8080
