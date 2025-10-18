# IPPAN Production Deployment Guide

## 🎯 **Deploy to Production Servers**

### **Server Configuration:**
- **Server 1**: 188.245.97.41 (Node 1 - Port 8080, P2P 9000)
- **Server 2**: 135.181.145.174 (Node 2 - Port 8081, P2P 9001)

---

## 🚀 **Deployment Steps**

### **Step 1: Deploy to Server 1**

```bash
# Copy deployment script to Server 1
scp deploy-server1.sh root@188.245.97.41:/root/

# SSH to Server 1 and run deployment
ssh root@188.245.97.41
chmod +x /root/deploy-server1.sh
/root/deploy-server1.sh
```

### **Step 2: Deploy to Server 2**

```bash
# Copy deployment script to Server 2
scp deploy-server2.sh root@135.181.145.174:/root/

# SSH to Server 2 and run deployment
ssh root@135.181.145.174
chmod +x /root/deploy-server2.sh
/root/deploy-server2.sh
```

---

## 🔍 **Verification Commands**

### **Check Node Health:**
```bash
# Node 1 Health
curl http://188.245.97.41:8080/health

# Node 2 Health
curl http://135.181.145.174:8081/health
```

### **Check P2P Connections:**
```bash
# Node 1 Peers
curl http://188.245.97.41:8080/p2p/peers

# Node 2 Peers
curl http://135.181.145.174:8081/p2p/peers
```

### **Check Node Status:**
```bash
# Node 1 Status
curl http://188.245.97.41:8080/status

# Node 2 Status
curl http://135.181.145.174:8081/status
```

---

## 🏥 **Health Monitoring**

### **On Each Server:**
```bash
# Check Docker containers
docker ps

# Check logs
docker compose logs ippan-node

# Check firewall
ufw status
```

---

## 🔧 **Troubleshooting**

### **If Node Won't Start:**
```bash
# Check logs
docker compose logs ippan-node

# Restart node
docker compose down
docker compose up -d
```

### **If Health Check Fails:**
```bash
# Check if ports are open
netstat -tlnp | grep :8080
netstat -tlnp | grep :9000

# Check firewall
ufw status
```

### **If P2P Connection Issues:**
```bash
# Test P2P ports
telnet 188.245.97.41 9000
telnet 135.181.145.174 9001
```

---

## 📊 **Expected Results**

After successful deployment:

✅ **Node 1**: `{"status":"healthy","node_id":"node-1","version":"0.1.0","peer_count":1}`
✅ **Node 2**: `{"status":"healthy","node_id":"node-2","version":"0.1.0","peer_count":1}`

Both nodes should show `peer_count: 1` indicating they're connected to each other.

---

## 🎉 **Deployment Complete!**

Once both nodes are running and healthy, you have successfully deployed the IPPAN blockchain network to production!

**Access Points:**
- **Node 1 API**: http://188.245.97.41:8080
- **Node 2 API**: http://135.181.145.174:8081
- **P2P Network**: Connected and operational
