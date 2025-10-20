# Server 1 Deployment Instructions

## 🎯 **Problem Solved: IPPAN Node Connectivity**

### **Current Status**
- ✅ **Server 2**: Fully operational and healthy
- ❌ **Server 1**: Not accessible via SSH (port 22 blocked)
- ❌ **Node Connectivity**: Not established (peer_count: 0)

### **Solution: Manual Server 1 Deployment**

Since Server 1 (188.245.97.41) is not accessible via SSH, you need to deploy manually using one of these methods:

## **Method 1: Direct Server Access**

If you have physical or console access to Server 1:

```bash
# 1. Copy deployment files to Server 1
scp -r server1-deployment/ root@188.245.97.41:/opt/ippan/

# 2. SSH to Server 1 (when accessible)
ssh root@188.245.97.41

# 3. Run deployment
cd /opt/ippan
chmod +x deploy-server1.sh
./deploy-server1.sh

# 4. Verify deployment
docker ps
curl http://localhost:8080/health
```

## **Method 2: Alternative Server**

If Server 1 cannot be accessed, deploy to a different server:

1. **Choose a new server** with SSH access
2. **Update IP addresses** in the configuration
3. **Deploy using the same files**
4. **Update Server 2 bootstrap** to point to the new server

## **Method 3: Web-based Deployment**

If Server 1 has a web interface:

1. **Access the web interface** at http://188.245.97.41/
2. **Upload deployment files** through the web interface
3. **Execute deployment** via web terminal or file manager
4. **Configure firewall** to allow ports 8080 and 9000

## **Verification Steps**

After deployment, verify the setup:

```powershell
# Run connectivity test
.\test-node-connectivity.ps1

# Expected results:
# - Server 1 API port 8080: Accessible
# - Server 1 P2P port 9000: Accessible  
# - Server 2 peer_count: > 0
# - Nodes connected and communicating
```

## **Firewall Configuration**

Ensure these ports are open on Server 1:

```bash
# Allow API port
ufw allow 8080/tcp

# Allow P2P port  
ufw allow 9000/tcp

# Reload firewall
ufw reload
```

## **Troubleshooting**

### **If deployment fails:**
1. Check Docker is installed: `docker --version`
2. Check Docker is running: `systemctl status docker`
3. Check ports are available: `netstat -tlnp | grep -E ':(8080|9000)'`
4. Check firewall: `ufw status`

### **If nodes don't connect:**
1. Verify both servers are accessible
2. Check P2P ports are open (9000, 9001)
3. Check bootstrap configuration
4. Review container logs: `docker logs ippan-node-1`

## **Success Criteria**

✅ **Both servers accessible via API**
✅ **P2P ports open and listening**
✅ **Peer count > 0 in health checks**
✅ **Nodes can communicate and sync blocks**

## **Next Steps**

Once both nodes are deployed and connected:

1. **Monitor node health** regularly
2. **Set up automated monitoring**
3. **Configure backup procedures**
4. **Plan for additional nodes**
5. **Implement load balancing**

---

**The IPPAN blockchain network will be fully operational once Server 1 is deployed and both nodes can communicate!** 🚀
