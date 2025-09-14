# IPPAN Multi-Node Deployment Solution

## Current Status
✅ **Both servers are reachable**  
❌ **SSH authentication needs to be set up**  
❌ **IPPAN services are not deployed yet**

## SSH Public Key Generated
Your SSH public key has been generated and is ready to use:

```
ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQDv7mfvbhDRfBUaCknJ2BP9JEUeU3RT5RKxTsMaAhtdJqvRyz6UchEgkvzMzXbt/w1kvfEcK4ev8ial+y5StTPME6SF95syjvtsNTDhr/cHSUZsV3Nbb6w7wTQGn42O1HV7o4L8Q2Fu+zbNLiHp3PXi8dONlOHptKQif/bNCxRf2uanNwPGVwGHslvEYVNaB++OcVCICmjev/rI8Bx5NZMAz4uUOP7gRwRbTK5YZE8z7X/JFNZleRhaFse8xq2WYTa9rzarkrMagH+b0l/6yLP2qbth71GBMcIY/Az3WJfumyhD5w/EkyzpqREs0kI3LbxxduuHUqDm7tK5FPIRZpSYwNJJ5adq3sx30XD7PbO3k+sh8/UtEtQwyodB9P2hhdpzszE1+TIVBauRohaQnQpwb0NBiE62qdVlN5RUXCV4j5LvFuJCeib/8m8d4H9HdHtA5H/2ZAya//1r5wwcNtgsRx/fagaLfsLMvyBhA/MKCCzrjsRl3HMj9UMOrJfKkZPIsb0W9CQkkpulsjsUlVe82ufh+sAT54niuC/HXZHeokGi51xIyq/ktfdzXoyfq+UBbSfbEIj3jOeyz75Avm7YucGoKuaI2CBghl9i4mXb+orNB1lxYzaBQc/ucilgWbMarP8bAZP9Qpy5TTyHeslodVCPEmyzCWUEts2iEkANGQ== yuyby@hugh
```

## Solution Options

### Option 1: Use Cloud-Init Files (Recommended)
1. **Update the cloud-init files** to include your SSH key
2. **Recreate the servers** with the updated cloud-init files
3. **Deploy IPPAN services** using the automated scripts

### Option 2: Manual SSH Key Setup
1. **SSH to each server** using password authentication (if available)
2. **Add the SSH key** to the authorized_keys file
3. **Deploy IPPAN services** using the automated scripts

### Option 3: Manual Deployment
1. **SSH to each server** using password authentication
2. **Manually deploy IPPAN** using the commands below

## Manual Deployment Commands

### For Server 1 (Nuremberg - 188.245.97.41):

```bash
# SSH to server
ssh ippan@188.245.97.41

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
```

### For Server 2 (Helsinki - 135.181.145.174):

```bash
# SSH to server
ssh ippan@135.181.145.174

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
```

## Verification

After deployment, run the verification script:

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

## Troubleshooting

1. **SSH Issues**: Ensure the SSH key is added to both servers
2. **Firewall**: Check that ports 8080, 3000, 9090 are open
3. **Docker**: Ensure Docker is running and accessible
4. **Logs**: Check `docker-compose logs -f` for any errors

## Next Steps

1. **Choose your deployment method** (cloud-init, manual SSH setup, or manual deployment)
2. **Deploy IPPAN services** on both servers
3. **Run verification script** to test the multi-node setup
4. **Monitor the network** using Grafana dashboards
