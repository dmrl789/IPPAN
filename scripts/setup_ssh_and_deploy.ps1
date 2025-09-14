# IPPAN Multi-Node Setup Script (PowerShell)
# This script helps set up SSH keys and deploy IPPAN on both servers

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

# Function to test basic connectivity
function Test-BasicConnectivity {
    param([string]$ServerIP, [string]$ServerName)
    
    Write-Status "Testing basic connectivity to $ServerName ($ServerIP)..."
    
    try {
        $ping = Test-Connection -ComputerName $ServerIP -Count 2 -Quiet
        if ($ping) {
            Write-Status "✅ $ServerName is reachable via ping"
            return $true
        } else {
            Write-Error "❌ $ServerName is not reachable via ping"
            return $false
        }
    } catch {
        Write-Error "❌ $ServerName is not reachable via ping"
        return $false
    }
}

# Function to check SSH connectivity
function Test-SSHConnectivity {
    param([string]$ServerIP, [string]$ServerName)
    
    Write-Status "Testing SSH connectivity to $ServerName..."
    
    try {
        $sshTest = ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no -o PasswordAuthentication=no $IPPAN_USER@$ServerIP "echo 'SSH test successful'" 2>&1
        
        if ($LASTEXITCODE -eq 0) {
            Write-Status "✅ SSH to $ServerName is working"
            return $true
        } else {
            Write-Warning "⚠️  SSH to $ServerName failed: $sshTest"
            return $false
        }
    } catch {
        Write-Warning "⚠️  SSH to $ServerName failed"
        return $false
    }
}

# Function to generate SSH key if it doesn't exist
function New-SSHKeyIfNeeded {
    Write-Header "🔑 Setting up SSH Key"
    
    $sshKeyPath = "$env:USERPROFILE\.ssh\id_rsa"
    
    if (-not (Test-Path $sshKeyPath)) {
        Write-Status "Generating new SSH key..."
        try {
            ssh-keygen -t rsa -b 4096 -f $sshKeyPath -N '""' -C "ippan-deployment"
            Write-Status "✅ SSH key generated successfully"
        } catch {
            Write-Error "❌ Failed to generate SSH key"
            return $false
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
    
    return $true
}

# Function to create cloud-init files with SSH key
function New-CloudInitWithSSH {
    param([string]$PublicKey)
    
    Write-Header "📝 Creating Cloud-Init Files with SSH Key"
    
    # Read the original cloud-init files
    $node1Content = Get-Content "ippan-cloud-init.yml" -Raw
    $node2Content = Get-Content "ippan-cloud-init-node2.yml" -Raw
    
    # Add SSH key to users section
    $sshKeySection = @"

  # SSH Key for deployment access
  - name: ippan
    groups: [sudo, docker]
    shell: /bin/bash
    sudo: ['ALL=(ALL) NOPASSWD:ALL']
    lock_passwd: false
    passwd: `$6`$rounds=4096`$salt`$hashed_password_here
    ssh_authorized_keys:
      - $PublicKey
"@

    # Update node1 cloud-init
    $node1Updated = $node1Content -replace "(users:\s*-\s*name:\s*ippan[^}]+})", $sshKeySection
    $node1Updated | Out-File -FilePath "ippan-cloud-init-node1-with-ssh.yml" -Encoding UTF8
    
    # Update node2 cloud-init  
    $node2Updated = $node2Content -replace "(users:\s*-\s*name:\s*ippan[^}]+})", $sshKeySection
    $node2Updated | Out-File -FilePath "ippan-cloud-init-node2-with-ssh.yml" -Encoding UTF8
    
    Write-Status "✅ Created cloud-init files with SSH key"
    Write-Status "Files created:"
    Write-Host "  - ippan-cloud-init-node1-with-ssh.yml" -ForegroundColor Cyan
    Write-Host "  - ippan-cloud-init-node2-with-ssh.yml" -ForegroundColor Cyan
}

# Function to create manual deployment instructions
function New-ManualDeploymentInstructions {
    Write-Header "📋 Manual Deployment Instructions"
    
    $instructions = @"
# Manual IPPAN Deployment Instructions

## Prerequisites
1. Both servers should be running Ubuntu/Debian
2. SSH access configured for the 'ippan' user
3. Docker and Docker Compose installed

## Step 1: Set up SSH Access

### Option A: Use Cloud-Init Files (Recommended)
1. Use the generated cloud-init files with SSH keys:
   - ippan-cloud-init-node1-with-ssh.yml (for Server 1)
   - ippan-cloud-init-node2-with-ssh.yml (for Server 2)

2. Recreate your servers with these cloud-init files

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

### On Server 1 (Nuremberg - 188.245.97.41):
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

### On Server 2 (Helsinki - 135.181.145.174):
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

## Step 3: Verify Deployment

Run the verification script:
```powershell
powershell -ExecutionPolicy Bypass -File scripts/verify_multi_node_deployment.ps1
```

## Expected Results

After successful deployment:
- Server 1 API: http://188.245.97.41:3000
- Server 2 API: http://135.181.145.174:3000
- Both servers should be connected and participating in consensus

## Troubleshooting

1. **SSH Issues**: Ensure SSH keys are properly configured
2. **Firewall**: Check that ports 8080, 3000, 9090 are open
3. **Docker**: Ensure Docker is running and accessible
4. **Logs**: Check `docker-compose logs -f` for any errors
"@

    $instructions | Out-File -FilePath "MANUAL_DEPLOYMENT_INSTRUCTIONS.md" -Encoding UTF8
    Write-Status "✅ Created manual deployment instructions"
    Write-Status "File created: MANUAL_DEPLOYMENT_INSTRUCTIONS.md"
}

# Main function
function Main {
    Write-Header "🚀 IPPAN Multi-Node Setup Assistant"
    Write-Host "Server 1 (Nuremberg): $SERVER1_IP"
    Write-Host "Server 2 (Helsinki): $SERVER2_IP"
    Write-Host "================================================"
    
    # Test basic connectivity
    Write-Header "🔍 Testing Basic Connectivity"
    $server1Reachable = Test-BasicConnectivity -ServerIP $SERVER1_IP -ServerName "Server 1"
    $server2Reachable = Test-BasicConnectivity -ServerIP $SERVER2_IP -ServerName "Server 2"
    
    if (-not $server1Reachable -or -not $server2Reachable) {
        Write-Error "❌ One or both servers are not reachable"
        Write-Warning "Please check server status and network connectivity"
        return
    }
    
    # Test SSH connectivity
    Write-Header "🔑 Testing SSH Connectivity"
    $server1SSH = Test-SSHConnectivity -ServerIP $SERVER1_IP -ServerName "Server 1"
    $server2SSH = Test-SSHConnectivity -ServerIP $SERVER2_IP -ServerName "Server 2"
    
    if (-not $server1SSH -or -not $server2SSH) {
        Write-Warning "⚠️  SSH access is not configured properly"
        
        # Generate SSH key
        if (New-SSHKeyIfNeeded) {
            $publicKey = Get-Content "$env:USERPROFILE\.ssh\id_rsa.pub" -ErrorAction SilentlyContinue
            if ($publicKey) {
                New-CloudInitWithSSH -PublicKey $publicKey
                New-ManualDeploymentInstructions
                
                Write-Header "📋 Next Steps"
                Write-Status "1. Use the generated cloud-init files to recreate your servers"
                Write-Status "2. Or manually add the SSH key to both servers"
                Write-Status "3. Then run the deployment scripts"
                Write-Status ""
                Write-Status "Files created:"
                Write-Host "  - ippan-cloud-init-node1-with-ssh.yml" -ForegroundColor Cyan
                Write-Host "  - ippan-cloud-init-node2-with-ssh.yml" -ForegroundColor Cyan
                Write-Host "  - MANUAL_DEPLOYMENT_INSTRUCTIONS.md" -ForegroundColor Cyan
            }
        }
    } else {
        Write-Status "✅ SSH access is working on both servers"
        Write-Status "You can now run the deployment scripts:"
        Write-Host "  ./scripts/deploy_server1.sh" -ForegroundColor Cyan
        Write-Host "  ./scripts/deploy_server2_connect.sh" -ForegroundColor Cyan
    }
}

# Run main function
Main
