# IPPAN Multi-Node Deployment via Hetzner Cloud Console

## Current Status
❌ **SSH access not working** - Need to set up via Hetzner Cloud Console  
✅ **Both servers are reachable** (188.245.97.41 and 135.181.145.174)  
✅ **SSH key generated and ready**  
✅ **All deployment scripts prepared**

## Your SSH Public Key
```
ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQDv7mfvbhDRfBUaCknJ2BP9JEUeU3RT5RKxTsMaAhtdJqvRyz6UchEgkvzMzXbt/w1kvfEcK4ev8ial+y5StTPME6SF95syjvtsNTDhr/cHSUZsV3Nbb6w7wTQGn42O1HV7o4L8Q2Fu+zbNLiHp3PXi8dONlOHptKQif/bNCxRf2uanNwPGVwGHslvEYVNaB++OcVCICmjev/rI8Bx5NZMAz4uUOP7gRwRbTK5YZE8z7X/JFNZleRhaFse8xq2WYTa9rzarkrMagH+b0l/6yLP2qbth71GBMcIY/Az3WJfumyhD5w/EkyzpqREs0kI3LbxxduuHUqDm7tK5FPIRZpSYwNJJ5adq3sx30XD7PbO3k+sh8/UtEtQwyodB9P2hhdpzszE1+TIVBauRohaQnQpwb0NBiE62qdVlN5RUXCV4j5LvFuJCeib/8m8d4H9HdHtA5H/2ZAya//1r5wwcNtgsRx/fagaLfsLMvyBhA/MKCCzrjsRl3HMj9UMOrJfKkZPIsb0W9CQkkpulsjsUlVe82ufh+sAT54niuC/HXZHeokGi51xIyq/ktfdzXoyfq+UBbSfbEIj3jOeyz75Avm7YucGoKuaI2CBghl9i4mXb+orNB1lxYzaBQc/ucilgWbMarP8bAZP9Qpy5TTyHeslodVCPEmyzCWUEts2iEkANGQ== yuyby@hugh
```

## Deployment Options

### Option 1: Recreate Servers with Updated Cloud-Init (Recommended)

**Steps:**
1. **Access Hetzner Cloud Console** (https://console.hetzner.cloud/)
2. **Delete existing servers** (if you want to start fresh)
3. **Create new servers** using the updated cloud-init files:
   - Use `ippan-cloud-init-node1-with-ssh.yml` for Server 1
   - Use `ippan-cloud-init-node2-with-ssh.yml` for Server 2
4. **Wait for servers to initialize** (5-10 minutes)
5. **Test SSH access** and deploy IPPAN services

### Option 2: Manual SSH Key Setup via Console

**Steps:**
1. **Access Hetzner Cloud Console** (https://console.hetzner.cloud/)
2. **Open server console** for both servers
3. **Add SSH key** to authorized_keys:
   ```bash
   mkdir -p ~/.ssh
   echo 'ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQDv7mfvbhDRfBUaCknJ2BP9JEUeU3RT5RKxTsMaAhtdJqvRyz6UchEgkvzMzXbt/w1kvfEcK4ev8ial+y5StTPME6SF95syjvtsNTDhr/cHSUZsV3Nbb6w7wTQGn42O1HV7o4L8Q2Fu+zbNLiHp3PXi8dONlOHptKQif/bNCxRf2uanNwPGVwGHslvEYVNaB++OcVCICmjev/rI8Bx5NZMAz4uUOP7gRwRbTK5YZE8z7X/JFNZleRhaFse8xq2WYTa9rzarkrMagH+b0l/6yLP2qbth71GBMcIY/Az3WJfumyhD5w/EkyzpqREs0kI3LbxxduuHUqDm7tK5FPIRZpSYwNJJ5adq3sx30XD7PbO3k+sh8/UtEtQwyodB9P2hhdpzszE1+TIVBauRohaQnQpwb0NBiE62qdVlN5RUXCV4j5LvFuJCeib/8m8d4H9HdHtA5H/2ZAya//1r5wwcNtgsRx/fagaLfsLMvyBhA/MKCCzrjsRl3HMj9UMOrJfKkZPIsb0W9CQkkpulsjsUlVe82ufh+sAT54niuC/HXZHeokGi51xIyq/ktfdzXoyfq+UBbSfbEIj3jOeyz75Avm7YucGoKuaI2CBghl9i4mXb+orNB1lxYzaBQc/ucilgWbMarP8bAZP9Qpy5TTyHeslodVCPEmyzCWUEts2iEkANGQ== yuyby@hugh' >> ~/.ssh/authorized_keys
   chmod 600 ~/.ssh/authorized_keys
   chmod 700 ~/.ssh
   ```

### Option 3: Direct Console Deployment

**Steps:**
1. **Access Hetzner Cloud Console** (https://console.hetzner.cloud/)
2. **Open server console** for both servers
3. **Run deployment commands directly** in the console

## Direct Console Deployment Commands

### For Server 1 (Nuremberg - 188.245.97.41):

```bash
# Switch to ippan user
sudo su - ippan

# Clone IPPAN repository
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

# Check status
docker ps
```

### For Server 2 (Helsinki - 135.181.145.174):

```bash
# Switch to ippan user
sudo su - ippan

# Clone IPPAN repository
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

# Check status
docker ps
```

## Verification Commands

After deployment, run these commands in the console to verify:

```bash
# Check Docker services
docker ps

# Check IPPAN logs
docker-compose logs -f

# Test API endpoints
curl http://localhost:3000/health

# Check network connectivity
curl http://188.245.97.41:3000/health  # From Server 2
curl http://135.181.145.174:3000/health  # From Server 1

# Check peer connections
curl http://localhost:3000/api/v1/network/peers
```

## Expected Results

After successful deployment:
- **Server 1 API**: http://188.245.97.41:3000
- **Server 2 API**: http://135.181.145.174:3000
- **Server 1 Grafana**: http://188.245.97.41:3001
- **Server 2 Grafana**: http://135.181.145.174:3001
- **Both servers connected** and participating in consensus

## Troubleshooting

1. **Docker Issues**: Ensure Docker is running: `sudo systemctl start docker`
2. **Firewall**: Check that ports 8080, 3000, 9090 are open
3. **Logs**: Check `docker-compose logs -f` for any errors
4. **Network**: Verify both servers can reach each other on port 8080
5. **Permissions**: Ensure ippan user has proper permissions

## Monitoring Commands

```bash
# View real-time logs
docker-compose logs -f

# Check service status
docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"

# Monitor system resources
htop

# Check network connections
netstat -tlnp | grep -E ':(8080|3000|9090)'

# Test API endpoints
curl -s http://localhost:3000/health | jq .
curl -s http://localhost:3000/api/v1/node/info | jq .
curl -s http://localhost:3000/api/v1/network/peers | jq .
```

## Next Steps

1. **Choose your deployment method** (recreate servers, manual SSH setup, or direct console)
2. **Deploy IPPAN services** on both servers using the commands above
3. **Verify deployment** using the verification commands
4. **Monitor the network** using the monitoring commands
5. **Test the multi-node setup** by checking peer connections

## Files Available

- `ippan-cloud-init-node1-with-ssh.yml` - Server 1 cloud-init with SSH key
- `ippan-cloud-init-node2-with-ssh.yml` - Server 2 cloud-init with SSH key
- `scripts/verify_multi_node_deployment.ps1` - Verification script (run after SSH is working)
- `FINAL_DEPLOYMENT_GUIDE.md` - Complete deployment guide
- `MULTI_NODE_DEPLOYMENT_GUIDE.md` - Comprehensive deployment guide
