# IPPAN Multi-Node Final Deployment Guide

## Current Status
✅ **Both servers are reachable** (188.245.97.41 and 135.181.145.174)  
✅ **SSH key generated and ready**  
✅ **Updated cloud-init files created**  
❌ **SSH access needs to be configured**  
❌ **IPPAN services not deployed yet**

## Your SSH Public Key
```
ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQDv7mfvbhDRfBUaCknJ2BP9JEUeU3RT5RKxTsMaAhtdJqvRyz6UchEgkvzMzXbt/w1kvfEcK4ev8ial+y5StTPME6SF95syjvtsNTDhr/cHSUZsV3Nbb6w7wTQGn42O1HV7o4L8Q2Fu+zbNLiHp3PXi8dONlOHptKQif/bNCxRf2uanNwPGVwGHslvEYVNaB++OcVCICmjev/rI8Bx5NZMAz4uUOP7gRwRbTK5YZE8z7X/JFNZleRhaFse8xq2WYTa9rzarkrMagH+b0l/6yLP2qbth71GBMcIY/Az3WJfumyhD5w/EkyzpqREs0kI3LbxxduuHUqDm7tK5FPIRZpSYwNJJ5adq3sx30XD7PbO3k+sh8/UtEtQwyodB9P2hhdpzszE1+TIVBauRohaQnQpwb0NBiE62qdVlN5RUXCV4j5LvFuJCeib/8m8d4H9HdHtA5H/2ZAya//1r5wwcNtgsRx/fagaLfsLMvyBhA/MKCCzrjsRl3HMj9UMOrJfKkZPIsb0W9CQkkpulsjsUlVe82ufh+sAT54niuC/HXZHeokGi51xIyq/ktfdzXoyfq+UBbSfbEIj3jOeyz75Avm7YucGoKuaI2CBghl9i4mXb+orNB1lxYzaBQc/ucilgWbMarP8bAZP9Qpy5TTyHeslodVCPEmyzCWUEts2iEkANGQ== yuyby@hugh
```

## Deployment Options

### Option 1: Recreate Servers with Updated Cloud-Init (Recommended)

**Files Created:**
- `ippan-cloud-init-node1-with-ssh.yml` - Server 1 with your SSH key
- `ippan-cloud-init-node2-with-ssh.yml` - Server 2 with your SSH key

**Steps:**
1. **Access Hetzner Cloud Console**
2. **Delete existing servers** (if you want to start fresh)
3. **Create new servers** using the updated cloud-init files
4. **Wait for servers to initialize** (5-10 minutes)
5. **Test SSH access** and deploy IPPAN services

### Option 2: Manual SSH Key Setup

**Steps:**
1. **Access Hetzner Cloud Console**
2. **Open server console** for both servers
3. **Add SSH key** to authorized_keys:
   ```bash
   mkdir -p ~/.ssh
   echo 'ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQDv7mfvbhDRfBUaCknJ2BP9JEUeU3RT5RKxTsMaAhtdJqvRyz6UchEgkvzMzXbt/w1kvfEcK4ev8ial+y5StTPME6SF95syjvtsNTDhr/cHSUZsV3Nbb6w7wTQGn42O1HV7o4L8Q2Fu+zbNLiHp3PXi8dONlOHptKQif/bNCxRf2uanNwPGVwGHslvEYVNaB++OcVCICmjev/rI8Bx5NZMAz4uUOP7gRwRbTK5YZE8z7X/JFNZleRhaFse8xq2WYTa9rzarkrMagH+b0l/6yLP2qbth71GBMcIY/Az3WJfumyhD5w/EkyzpqREs0kI3LbxxduuHUqDm7tK5FPIRZpSYwNJJ5adq3sx30XD7PbO3k+sh8/UtEtQwyodB9P2hhdpzszE1+TIVBauRohaQnQpwb0NBiE62qdVlN5RUXCV4j5LvFuJCeib/8m8d4H9HdHtA5H/2ZAya//1r5wwcNtgsRx/fagaLfsLMvyBhA/MKCCzrjsRl3HMj9UMOrJfKkZPIsb0W9CQkkpulsjsUlVe82ufh+sAT54niuC/HXZHeokGi51xIyq/ktfdzXoyfq+UBbSfbEIj3jOeyz75Avm7YucGoKuaI2CBghl9i4mXb+orNB1lxYzaBQc/ucilgWbMarP8bAZP9Qpy5TTyHeslodVCPEmyzCWUEts2iEkANGQ== yuyby@hugh' >> ~/.ssh/authorized_keys
   chmod 600 ~/.ssh/authorized_keys
   chmod 700 ~/.ssh
   ```

## Deploy IPPAN Services

### Step 1: Test SSH Access
```bash
ssh ippan@188.245.97.41
ssh ippan@135.181.145.174
```

### Step 2: Deploy Server 1 (Nuremberg)
```bash
# SSH to Server 1
ssh ippan@188.245.97.41

# Clone and setup IPPAN
cd /opt/ippan
git clone https://github.com/dmrl789/IPPAN.git ippan-repo
cp -r ippan-repo/* mainnet/
rm -rf ippan-repo

# Create node1 configuration
cat > mainnet/config.toml << 'EOF'
[network]
bootstrap_nodes = [
    "188.245.97.41:8080",  # Node 1 (Nuremberg)
    "135.181.145.174:8080" # Node 2 (Helsinki)
]
listen_address = "0.0.0.0:8080"
external_address = "188.245.97.41:8080"

[api]
listen_address = "0.0.0.0:3000"
cors_origins = ["*"]

[metrics]
listen_address = "0.0.0.0:9090"

[logging]
level = "info"
format = "json"

[consensus]
algorithm = "proof_of_stake"
block_time = 10
max_transactions_per_block = 1000

[storage]
data_dir = "/opt/ippan/data"
wal_dir = "/opt/ippan/wal"
EOF

# Deploy with Docker Compose
cd mainnet
docker-compose -f docker-compose.production.yml up -d
```

### Step 3: Deploy Server 2 (Helsinki)
```bash
# SSH to Server 2
ssh ippan@135.181.145.174

# Clone and setup IPPAN
cd /opt/ippan
git clone https://github.com/dmrl789/IPPAN.git ippan-repo
cp -r ippan-repo/* mainnet/
rm -rf ippan-repo

# Create node2 configuration
cat > mainnet/config.toml << 'EOF'
[network]
bootstrap_nodes = [
    "188.245.97.41:8080",  # Node 1 (Nuremberg)
    "135.181.145.174:8080" # Node 2 (Helsinki)
]
listen_address = "0.0.0.0:8080"
external_address = "135.181.145.174:8080"

[api]
listen_address = "0.0.0.0:3000"
cors_origins = ["*"]

[metrics]
listen_address = "0.0.0.0:9090"

[logging]
level = "info"
format = "json"

[consensus]
algorithm = "proof_of_stake"
block_time = 10
max_transactions_per_block = 1000

[storage]
data_dir = "/opt/ippan/data"
wal_dir = "/opt/ippan/wal"
EOF

# Deploy with Docker Compose
cd mainnet
docker-compose -f docker-compose.production.yml up -d
```

### Step 4: Verify Deployment
```powershell
powershell -ExecutionPolicy Bypass -File scripts/verify_multi_node_deployment.ps1
```

## Expected Results

After successful deployment:
- **Server 1 API**: http://188.245.97.41:3000
- **Server 2 API**: http://135.181.145.174:3000
- **Server 1 Grafana**: http://188.245.97.41:3001
- **Server 2 Grafana**: http://135.181.145.174:3001
- **Both servers connected** and participating in consensus

## Monitoring Commands

```bash
# Check service status
docker ps

# View logs
docker-compose logs -f

# Check IPPAN logs
tail -f /opt/ippan/mainnet/logs/*.log

# Test API endpoints
curl http://188.245.97.41:3000/health
curl http://135.181.145.174:3000/health

# Check peer connections
curl http://188.245.97.41:3000/api/v1/network/peers
curl http://135.181.145.174:3000/api/v1/network/peers
```

## Troubleshooting

1. **SSH Issues**: Ensure SSH key is properly added to both servers
2. **Firewall**: Check that ports 8080, 3000, 9090 are open
3. **Docker**: Ensure Docker is running and accessible
4. **Logs**: Check `docker-compose logs -f` for any errors
5. **Network**: Verify both servers can reach each other on port 8080

## Files Created

- `ippan-cloud-init-node1-with-ssh.yml` - Server 1 cloud-init with SSH key
- `ippan-cloud-init-node2-with-ssh.yml` - Server 2 cloud-init with SSH key
- `scripts/verify_multi_node_deployment.ps1` - Verification script
- `scripts/deploy_server1.sh` - Server 1 deployment script
- `scripts/deploy_server2_connect.sh` - Server 2 deployment script
- `DEPLOYMENT_SOLUTION.md` - Previous deployment guide
- `MULTI_NODE_DEPLOYMENT_GUIDE.md` - Comprehensive deployment guide

## Next Steps

1. **Choose your deployment method** (recreate servers or manual SSH setup)
2. **Set up SSH access** using one of the methods above
3. **Deploy IPPAN services** on both servers
4. **Run verification script** to test the multi-node setup
5. **Monitor the network** using Grafana dashboards
