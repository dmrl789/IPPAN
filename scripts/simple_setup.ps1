# Simple IPPAN Setup Script
# This script helps diagnose and set up the multi-node deployment

# Server configuration
$SERVER1_IP = "188.245.97.41"    # Nuremberg
$SERVER2_IP = "135.181.145.174"  # Helsinki
$IPPAN_USER = "ippan"

function Write-Status {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Green
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

function Write-Header {
    param([string]$Message)
    Write-Host "[HEADER] $Message" -ForegroundColor Blue
}

function Write-Warning {
    param([string]$Message)
    Write-Host "[WARNING] $Message" -ForegroundColor Yellow
}

# Test basic connectivity
Write-Header "🔍 Testing Basic Connectivity"

Write-Status "Testing Server 1 (Nuremberg) - $SERVER1_IP"
try {
    $ping1 = Test-Connection -ComputerName $SERVER1_IP -Count 2 -Quiet
    if ($ping1) {
        Write-Status "✅ Server 1 is reachable"
    } else {
        Write-Error "❌ Server 1 is not reachable"
    }
} catch {
    Write-Error "❌ Server 1 is not reachable"
}

Write-Status "Testing Server 2 (Helsinki) - $SERVER2_IP"
try {
    $ping2 = Test-Connection -ComputerName $SERVER2_IP -Count 2 -Quiet
    if ($ping2) {
        Write-Status "✅ Server 2 is reachable"
    } else {
        Write-Error "❌ Server 2 is not reachable"
    }
} catch {
    Write-Error "❌ Server 2 is not reachable"
}

# Test SSH connectivity
Write-Header "🔑 Testing SSH Connectivity"

Write-Status "Testing SSH to Server 1..."
try {
    $ssh1 = ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no $IPPAN_USER@$SERVER1_IP "echo 'SSH test successful'" 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Status "✅ SSH to Server 1 is working"
    } else {
        Write-Warning "⚠️  SSH to Server 1 failed: $ssh1"
    }
} catch {
    Write-Warning "⚠️  SSH to Server 1 failed"
}

Write-Status "Testing SSH to Server 2..."
try {
    $ssh2 = ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no $IPPAN_USER@$SERVER2_IP "echo 'SSH test successful'" 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Status "✅ SSH to Server 2 is working"
    } else {
        Write-Warning "⚠️  SSH to Server 2 failed: $ssh2"
    }
} catch {
    Write-Warning "⚠️  SSH to Server 2 failed"
}

# Generate SSH key if needed
Write-Header "🔑 SSH Key Setup"

$sshKeyPath = "$env:USERPROFILE\.ssh\id_rsa"
if (-not (Test-Path $sshKeyPath)) {
    Write-Status "Generating new SSH key..."
    try {
        ssh-keygen -t rsa -b 4096 -f $sshKeyPath -N '""' -C "ippan-deployment"
        Write-Status "✅ SSH key generated successfully"
    } catch {
        Write-Error "❌ Failed to generate SSH key"
    }
} else {
    Write-Status "✅ SSH key already exists"
}

# Display public key
$publicKey = Get-Content "$sshKeyPath.pub" -ErrorAction SilentlyContinue
if ($publicKey) {
    Write-Header "📋 Your SSH Public Key:"
    Write-Host $publicKey -ForegroundColor Cyan
    Write-Host ""
    Write-Warning "IMPORTANT: You need to add this public key to both servers!"
    Write-Warning "1. Copy the key above"
    Write-Warning "2. SSH to each server and add it to ~/.ssh/authorized_keys"
    Write-Warning "3. Or use the cloud-init files to add it automatically"
}

# Create deployment instructions
Write-Header "📋 Creating Deployment Instructions"

$instructions = @"
# IPPAN Multi-Node Deployment Instructions

## Current Status
- Server 1 (Nuremberg): $SERVER1_IP
- Server 2 (Helsinki): $SERVER2_IP

## Prerequisites
1. Both servers should be running Ubuntu/Debian
2. SSH access configured for the 'ippan' user
3. Docker and Docker Compose installed

## Step 1: Set up SSH Access

### Option A: Use Cloud-Init Files (Recommended)
1. Use the existing cloud-init files:
   - ippan-cloud-init.yml (for Server 1)
   - ippan-cloud-init-node2.yml (for Server 2)

2. Add your SSH public key to the users section in both files

### Option B: Manual SSH Key Setup
1. SSH to each server using password authentication
2. Add your public key to ~/.ssh/authorized_keys:
   ```bash
   mkdir -p ~/.ssh
   echo 'YOUR_PUBLIC_KEY_HERE' >> ~/.ssh/authorized_keys
   chmod 600 ~/.ssh/authorized_keys
   chmod 700 ~/.ssh
   ```

## Step 2: Deploy IPPAN Services

### On Server 1 (Nuremberg - $SERVER1_IP):
```bash
# Clone repository
cd /opt/ippan
git clone https://github.com/dmrl789/IPPAN.git ippan-repo
cp -r ippan-repo/* mainnet/
rm -rf ippan-repo

# Create configuration
cat > mainnet/config.toml << 'EOF'
[network]
bootstrap_nodes = [
    "$SERVER1_IP:8080",  # Node 1 (Nuremberg)
    "$SERVER2_IP:8080"   # Node 2 (Helsinki)
]
listen_address = "0.0.0.0:8080"
external_address = "$SERVER1_IP:8080"

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

### On Server 2 (Helsinki - $SERVER2_IP):
```bash
# Clone repository
cd /opt/ippan
git clone https://github.com/dmrl789/IPPAN.git ippan-repo
cp -r ippan-repo/* mainnet/
rm -rf ippan-repo

# Create configuration
cat > mainnet/config.toml << 'EOF'
[network]
bootstrap_nodes = [
    "$SERVER1_IP:8080",  # Node 1 (Nuremberg)
    "$SERVER2_IP:8080"   # Node 2 (Helsinki)
]
listen_address = "0.0.0.0:8080"
external_address = "$SERVER2_IP:8080"

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

## Step 3: Verify Deployment

Run the verification script:
```powershell
powershell -ExecutionPolicy Bypass -File scripts/verify_multi_node_deployment.ps1
```

## Expected Results

After successful deployment:
- Server 1 API: http://$SERVER1_IP:3000
- Server 2 API: http://$SERVER2_IP:3000
- Both servers should be connected and participating in consensus

## Troubleshooting

1. **SSH Issues**: Ensure SSH keys are properly configured
2. **Firewall**: Check that ports 8080, 3000, 9090 are open
3. **Docker**: Ensure Docker is running and accessible
4. **Logs**: Check `docker-compose logs -f` for any errors
"@

$instructions | Out-File -FilePath "DEPLOYMENT_INSTRUCTIONS.md" -Encoding UTF8
Write-Status "✅ Created deployment instructions: DEPLOYMENT_INSTRUCTIONS.md"

Write-Header "📋 Summary"
Write-Status "1. Check the connectivity test results above"
Write-Status "2. Set up SSH access using the public key shown"
Write-Status "3. Follow the instructions in DEPLOYMENT_INSTRUCTIONS.md"
Write-Status "4. Deploy IPPAN services on both servers"
Write-Status "5. Run the verification script to test the setup"
