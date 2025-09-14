# Manual IPPAN Deployment Steps

## Current Status
- SSH access to both servers needs to be set up
- IPPAN services need to be deployed
- Multi-node network needs to be configured

## Option 1: Use Hetzner Cloud Console (Recommended)

### Step 1: Access Hetzner Cloud Console
1. Go to https://console.hetzner.cloud/
2. Login to your account
3. Find your servers (188.245.97.41 and 135.181.145.174)

### Step 2: Open Server Console
1. Click on Server 1 (188.245.97.41)
2. Click "Console" tab
3. Login as root (or ippan if available)

### Step 3: Add SSH Key to Server 1
```bash
# Create ippan user if it doesn't exist
useradd -m -s /bin/bash -G sudo,docker ippan

# Create SSH directory
mkdir -p /home/ippan/.ssh

# Add your SSH public key
echo 'ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQDv7mfvbhDRfBUaCknJ2BP9JEUeU3RT5RKxTsMaAhtdJqvRyz6UchEgkvzMzXbt/w1kvfEcK4ev8ial+y5StTPME6SF95syjvtsNTDhr/cHSUZsV3Nbb6w7wTQGn42O1HV7o4L8Q2Fu+zbNLiHp3PXi8dONlOHptKQif/bNCxRf2uanNwPGVwGHslvEYVNaB++OcVCICmjev/rI8Bx5NZMAz4uUOP7gRwRbTK5YZE8z7X/JFNZleRhaFse8xq2WYTa9rzarkrMagH+b0l/6yLP2qbth71GBMcIY/Az3WJfumyhD5w/EkyzpqREs0kI3LbxxduuHUqDm7tK5FPIRZpSYwNJJ5adq3sx30XD7PbO3k+sh8/UtEtQwyodB9P2hhdpzszE1+TIVBauRohaQnQpwb0NBiE62qdVlN5RUXCV4j5LvFuJCeib/8m8d4H9HdHtA5H/2ZAya//1r5wwcNtgsRx/fagaLfsLMvyBhA/MKCCzrjsRl3HMj9UMOrJfKkZPIsb0W9CQkkpulsjsUlVe82ufh+sAT54niuC/HXZHeokGi51xIyq/ktfdzXoyfq+UBbSfbEIj3jOeyz75Avm7YucGoKuaI2CBghl9i4mXb+orNB1lxYzaBQc/ucilgWbMarP8bAZP9Qpy5TTyHeslodVCPEmyzCWUEts2iEkANGQ== yuyby@hugh' >> /home/ippan/.ssh/authorized_keys

# Set proper permissions
chown -R ippan:ippan /home/ippan/.ssh
chmod 700 /home/ippan/.ssh
chmod 600 /home/ippan/.ssh/authorized_keys
```

### Step 4: Deploy IPPAN on Server 1
```bash
# Switch to ippan user
su - ippan

# Install dependencies
sudo apt update && sudo apt install -y curl git ufw fail2ban ca-certificates gnupg lsb-release

# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh
sudo usermod -aG docker $USER

# Install Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose

# Configure firewall
sudo ufw --force reset
sudo ufw default deny incoming && sudo ufw default allow outgoing
sudo ufw allow 22,80,443,3000,8080,9090,3001/tcp
sudo ufw --force enable

# Clone and setup IPPAN
mkdir -p /opt/ippan && cd /opt/ippan
git clone https://github.com/dmrl789/IPPAN.git mainnet
cd mainnet

# Create node1 configuration
cat > multi-node-node1.toml << 'EOF'
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

# Make config discoverable
mkdir -p config && cp multi-node-node1.toml config/node.toml

# Start IPPAN services
docker-compose -f docker-compose.production.yml up -d

# Check status
docker ps
```

### Step 5: Repeat for Server 2
1. Open Server 2 console (135.181.145.174)
2. Run the same SSH key setup commands
3. Run the same deployment commands, but use this config for Server 2:

```bash
# Create node2 configuration
cat > multi-node-node2.toml << 'EOF'
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

# Make config discoverable
mkdir -p config && cp multi-node-node2.toml config/node.toml

# Start IPPAN services
docker-compose -f docker-compose.production.yml up -d

# Check status
docker ps
```

## Step 6: Verify Deployment

After both servers are deployed, test from your laptop:

```powershell
# Test Server 1
ssh ippan@188.245.97.41 "curl -sf http://127.0.0.1:3000/health || echo 'API not ready'"

# Test Server 2
ssh ippan@135.181.145.174 "curl -sf http://127.0.0.1:3000/health || echo 'API not ready'"

# Check cross-connectivity
ssh ippan@188.245.97.41 "curl -sf http://135.181.145.174:3000/health || echo 'Server2 not reachable'"
ssh ippan@135.181.145.174 "curl -sf http://188.245.97.41:3000/health || echo 'Server1 not reachable'"
```

## Expected Results

After successful deployment:
- **Server 1 API**: http://188.245.97.41:3000
- **Server 2 API**: http://135.181.145.174:3000
- **Server 1 Grafana**: http://188.245.97.41:3001
- **Server 2 Grafana**: http://135.181.145.174:3001
- **Both servers connected** and participating in consensus

## Troubleshooting

1. **SSH Issues**: Ensure the SSH key is properly added to both servers
2. **Firewall**: Check that ports 8080, 3000, 9090 are open
3. **Docker**: Ensure Docker is running and accessible
4. **Logs**: Check `docker-compose logs -f` for any errors
5. **Network**: Verify both servers can reach each other on port 8080

## Next Steps

1. **Follow the manual steps above** using Hetzner Cloud Console
2. **Deploy IPPAN services** on both servers
3. **Test the multi-node setup** using the verification commands
4. **Monitor the network** using Grafana dashboards
