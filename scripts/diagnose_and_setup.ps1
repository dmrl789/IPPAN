# IPPAN Multi-Node Diagnosis and Setup Script

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
        ssh-keygen -t rsa -b 4096 -f $sshKeyPath -N '""' -C 'ippan-deployment'
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

Write-Header "📋 Next Steps"
Write-Status "1. Set up SSH access using the public key shown above"
Write-Status "2. Once SSH is working, you can run the deployment scripts"
Write-Status "3. Or follow the manual deployment instructions"

Write-Header "📋 Manual Deployment Commands"

Write-Status "For Server 1 (Nuremberg):"
Write-Host "ssh $IPPAN_USER@$SERVER1_IP" -ForegroundColor Cyan
Write-Host "cd /opt/ippan && git clone https://github.com/dmrl789/IPPAN.git ippan-repo" -ForegroundColor Cyan
Write-Host "cp -r ippan-repo/* mainnet/ && rm -rf ippan-repo" -ForegroundColor Cyan
Write-Host "cd mainnet && docker-compose -f docker-compose.production.yml up -d" -ForegroundColor Cyan

Write-Status "For Server 2 (Helsinki):"
Write-Host "ssh $IPPAN_USER@$SERVER2_IP" -ForegroundColor Cyan
Write-Host "cd /opt/ippan && git clone https://github.com/dmrl789/IPPAN.git ippan-repo" -ForegroundColor Cyan
Write-Host "cp -r ippan-repo/* mainnet/ && rm -rf ippan-repo" -ForegroundColor Cyan
Write-Host "cd mainnet && docker-compose -f docker-compose.production.yml up -d" -ForegroundColor Cyan

Write-Status "After deployment, run verification:"
Write-Host "powershell -ExecutionPolicy Bypass -File scripts/verify_multi_node_deployment.ps1" -ForegroundColor Cyan
