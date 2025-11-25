# 🚀 IPPAN Production Deployment Guide

## ✅ **DEPLOYMENT READY STATUS**

**YES, the IPPAN blockchain is now ready for server deployment!** 

All critical compilation errors have been resolved, and the system builds successfully in release mode. The Production Upgrade Pack includes:

- ✅ **Real Axum HTTP server** with live RPC endpoints
- ✅ **Sled-backed persistent storage** for blocks/accounts  
- ✅ **Minimal PoA slot consensus** that produces blocks
- ✅ **Simplified P2P module** (ready for libp2p integration)
- ✅ **Production Docker configuration**
- ✅ **Systemd service unit**
- ✅ **CI/CD workflow**
- ✅ **Environment configuration**
- ✅ **Security & audit coordination playbook** (`docs/SECURITY_AND_AUDIT_PLAYBOOK.md`)

---

## 🏃‍♂️ **Quick Deployment Options**

### **Option 1: Docker Deployment (Recommended)**

```bash
# 1. Build and run with Docker Compose
docker-compose -f deploy/docker-compose.production.yml up -d

# 2. Check status
docker-compose -f deploy/docker-compose.production.yml ps

# 3. View logs
docker-compose -f deploy/docker-compose.production.yml logs -f ippan-node

# 4. Test API
curl http://localhost:8080/health
curl http://localhost:8080/status
```

### **Option 2: Bare Metal Deployment**

```bash
# 1. Build the project
cargo build --release --workspace

# 2. Create system user
sudo useradd -r -s /bin/false ippan

# 3. Create directories
sudo mkdir -p /opt/ippan/bin /var/lib/ippan /var/log/ippan /etc/ippan
sudo chown -R ippan:ippan /var/lib/ippan /var/log/ippan

# 4. Copy binary
sudo cp target/release/ippan-node /opt/ippan/bin/

# 5. Copy configuration
sudo cp config/ippan.env.example /etc/ippan/ippan.env
sudo chown ippan:ippan /etc/ippan/ippan.env

# 6. Install systemd service
sudo cp deploy/ippan-node.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable ippan-node
sudo systemctl start ippan-node

# 7. Check status
sudo systemctl status ippan-node
sudo journalctl -u ippan-node -f
```

### **Option 3: Local Development**

```bash
# 1. Build
cargo build --release --workspace

# 2. Run with local config
RUST_LOG=debug cargo run --bin ippan-node -- --dev

# 3. Test API
curl http://localhost:8080/health
curl http://localhost:8080/status
```

---

## 🔧 **Configuration**

### **Environment Variables**

Copy `config/ippan.env.example` to your deployment location and customize:

```bash
# Node Identity
NODE_ID=ippan_node_001
VALIDATOR_ID=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef

# Network Configuration  
RPC_HOST=0.0.0.0
RPC_PORT=8080
P2P_HOST=0.0.0.0
P2P_PORT=9000

# Storage Configuration
DATA_DIR=/var/lib/ippan
DB_PATH=/var/lib/ippan/db

# Consensus Configuration
SLOT_DURATION_MS=1000
MAX_TRANSACTIONS_PER_BLOCK=1000
BLOCK_REWARD=10

# Logging
LOG_LEVEL=info
LOG_FORMAT=json
```

---

## 📊 **API Endpoints**

Once deployed, the following endpoints are available:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/status` | Node status and metrics |
| `/time` | GET | Current IPPAN time |
| `/block?hash=<hash>` | GET | Get block by hash |
| `/block?height=<height>` | GET | Get block by height |
| `/tx` | POST | Submit transaction |
| `/tx/<hash>` | GET | Get transaction by hash |
| `/account/<address>` | GET | Get account info |
| `/accounts` | GET | List all accounts |

### **Example API Calls**

```bash
# Health check
curl http://localhost:8080/health

# Node status
curl http://localhost:8080/status

# Submit transaction
curl -X POST http://localhost:8080/tx \
  -H "Content-Type: application/json" \
  -d '{
    "from": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    "to": "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
    "amount": 1000,
    "nonce": 1,
    "signature": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
  }'

# Get account info
curl http://localhost:8080/account/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
```

---

## 🔍 **Monitoring & Logs**

### **Docker Logs**
```bash
# View all logs
docker-compose -f deploy/docker-compose.production.yml logs

# Follow logs
docker-compose -f deploy/docker-compose.production.yml logs -f

# View specific service logs
docker-compose -f deploy/docker-compose.production.yml logs ippan-node
```

### **Systemd Logs**
```bash
# View service status
sudo systemctl status ippan-node

# View logs
sudo journalctl -u ippan-node

# Follow logs
sudo journalctl -u ippan-node -f

# View recent logs
sudo journalctl -u ippan-node --since "1 hour ago"
```

---

## 🛡️ **Security Considerations**

### **Production Security Checklist**

- [ ] **Firewall**: Configure firewall to only allow necessary ports (8080, 9000)
- [ ] **SSL/TLS**: Enable TLS for RPC endpoints in production
- [ ] **User Permissions**: Run as non-root user (`ippan`)
- [ ] **File Permissions**: Restrict access to configuration and data files
- [ ] **Playbook Ready**: Keep `docs/SECURITY_AND_AUDIT_PLAYBOOK.md` handy for disclosure channels, incident coordination, and baseline hardening steps
- [ ] **Network Security**: Use VPN or private networks for P2P communication
- [ ] **Monitoring**: Set up log monitoring and alerting
- [ ] **Backup**: Implement regular database backups
- [ ] **Updates**: Plan for security updates and node upgrades

### **Docker Security**
```bash
# Run with security options
docker run --user 1000:1000 \
  --read-only \
  --tmpfs /tmp \
  --cap-drop=ALL \
  ippan-node:latest
```

---

## 🚨 **Troubleshooting**

### **Common Issues**

1. **Port Already in Use**
   ```bash
   # Check what's using the port
   sudo netstat -tlnp | grep :8080
   
   # Kill the process or change port in config
   ```

2. **Permission Denied**
   ```bash
   # Fix file permissions
   sudo chown -R ippan:ippan /var/lib/ippan
   sudo chmod 755 /var/lib/ippan
   ```

3. **Database Corruption**
   ```bash
   # Remove corrupted database (will recreate)
   sudo rm -rf /var/lib/ippan/db
   sudo systemctl restart ippan-node
   ```

4. **Memory Issues**
   ```bash
   # Check memory usage
   docker stats
   
   # Adjust memory limits in docker-compose.yml
   ```

### **Health Checks**

```bash
# Check if node is responding
curl -f http://localhost:8080/health || echo "Node is down"

# Check consensus status
curl http://localhost:8080/status | jq '.data.latest_height'

# Check storage
ls -la /var/lib/ippan/db/
```

---

## 📈 **Performance Tuning**

### **Production Optimizations**

1. **Database Tuning**
   ```bash
   # Increase database cache size
   export IPPAN_DB_CACHE_SIZE=1024
   ```

2. **Network Tuning**
   ```bash
   # Increase file descriptor limits
   ulimit -n 65536
   ```

3. **Memory Tuning**
   ```bash
   # Set JVM-style memory options (if applicable)
   export RUST_LOG=info
   ```

---

## 🔄 **Updates & Maintenance**

### **Updating the Node**

1. **Docker Update**
   ```bash
   docker-compose -f deploy/docker-compose.production.yml pull
   docker-compose -f deploy/docker-compose.production.yml up -d
   ```

2. **Bare Metal Update**
   ```bash
   # Build new version
   cargo build --release --workspace
   
   # Stop service
   sudo systemctl stop ippan-node
   
   # Backup current binary
   sudo cp /opt/ippan/bin/ippan-node /opt/ippan/bin/ippan-node.backup
   
   # Install new binary
   sudo cp target/release/ippan-node /opt/ippan/bin/
   
   # Start service
   sudo systemctl start ippan-node
   ```

---

## ✅ **Deployment Checklist**

- [ ] **Build successful**: `cargo build --release --workspace` passes
- [ ] **Configuration**: Environment variables configured
- [ ] **Storage**: Data directory created with proper permissions
- [ ] **Network**: Ports 8080 and 9000 accessible
- [ ] **Security**: Running as non-root user
- [ ] **Monitoring**: Log monitoring configured
- [ ] **Backup**: Database backup strategy in place
- [ ] **Health checks**: API endpoints responding
- [ ] **Consensus**: Blocks being produced (check `/status`)
- [ ] **Documentation**: Team trained on operations

---

## 🎯 **Next Steps**

1. **Deploy to staging environment** first
2. **Test all API endpoints** thoroughly  
3. **Monitor consensus behavior** and block production
4. **Set up monitoring and alerting**
5. **Plan for multi-node deployment**
6. **Implement libp2p networking** for full P2P functionality
7. **Add more sophisticated consensus algorithms**

---

**🎉 Congratulations! Your IPPAN blockchain node is ready for production deployment!**
