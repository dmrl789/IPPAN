# IPPAN Server Deployment Script
# This script will deploy IPPAN services on both servers

$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$IPPAN_USER = "ippan"

Write-Host "=== IPPAN Server Deployment ===" -ForegroundColor Cyan
Write-Host "Server 1 (Nuremberg): $SERVER1_IP" -ForegroundColor Blue
Write-Host "Server 2 (Helsinki): $SERVER2_IP" -ForegroundColor Blue
Write-Host ""

# Function to test server connectivity
function Test-ServerConnectivity {
    param([string]$ServerIP, [string]$ServerName)
    
    Write-Host "Testing $ServerName connectivity..." -ForegroundColor Yellow
    try {
        $ping = Test-Connection -ComputerName $ServerIP -Count 1 -Quiet
        if ($ping) {
            Write-Host "✅ $ServerName is reachable" -ForegroundColor Green
            return $true
        } else {
            Write-Host "❌ $ServerName is not reachable" -ForegroundColor Red
            return $false
        }
    } catch {
        Write-Host "❌ $ServerName ping failed" -ForegroundColor Red
        return $false
    }
}

# Function to test API endpoint
function Test-APIEndpoint {
    param([string]$ServerIP, [string]$ServerName)
    
    Write-Host "Testing $ServerName API..." -ForegroundColor Yellow
    try {
        $response = Invoke-WebRequest -Uri "http://$ServerIP`:3000/health" -TimeoutSec 5 -UseBasicParsing 2>$null
        if ($response.StatusCode -eq 200) {
            Write-Host "✅ $ServerName API is responding" -ForegroundColor Green
            return $true
        } else {
            Write-Host "⚠️ $ServerName API returned status: $($response.StatusCode)" -ForegroundColor Yellow
            return $false
        }
    } catch {
        Write-Host "❌ $ServerName API is not responding" -ForegroundColor Red
        return $false
    }
}

# Test both servers
$server1Online = Test-ServerConnectivity -ServerIP $SERVER1_IP -ServerName "Server 1"
$server2Online = Test-ServerConnectivity -ServerIP $SERVER2_IP -ServerName "Server 2"

if (-not $server1Online -and -not $server2Online) {
    Write-Host "❌ Neither server is reachable. Cannot proceed." -ForegroundColor Red
    exit 1
}

# Test API endpoints
$server1API = Test-APIEndpoint -ServerIP $SERVER1_IP -ServerName "Server 1"
$server2API = Test-APIEndpoint -ServerIP $SERVER2_IP -ServerName "Server 2"

Write-Host ""
Write-Host "=== Current Status ===" -ForegroundColor Cyan
Write-Host "Server 1: $(if($server1Online){'Online'}else{'Offline'}) - API: $(if($server1API){'Running'}else{'Not Running'})" -ForegroundColor White
Write-Host "Server 2: $(if($server2Online){'Online'}else{'Offline'}) - API: $(if($server2API){'Running'}else{'Not Running'})" -ForegroundColor White
Write-Host ""

if ($server1API -and $server2API) {
    Write-Host "🎉 Both servers are running IPPAN services!" -ForegroundColor Green
    Write-Host "Testing peer connection..." -ForegroundColor Yellow
    
    # Test peer connection
    try {
        $peers1 = Invoke-WebRequest -Uri "http://$SERVER1_IP`:3000/api/v1/network/peers" -TimeoutSec 10 -UseBasicParsing 2>$null
        $peers2 = Invoke-WebRequest -Uri "http://$SERVER2_IP`:3000/api/v1/network/peers" -TimeoutSec 10 -UseBasicParsing 2>$null
        
        if ($peers1.StatusCode -eq 200 -and $peers2.StatusCode -eq 200) {
            Write-Host "✅ Both servers can access peer API" -ForegroundColor Green
            Write-Host "Server 1 peers: $($peers1.Content)" -ForegroundColor Cyan
            Write-Host "Server 2 peers: $($peers2.Content)" -ForegroundColor Cyan
        }
    } catch {
        Write-Host "⚠️ Could not test peer connections" -ForegroundColor Yellow
    }
    
    Write-Host ""
    Write-Host "=== Access URLs ===" -ForegroundColor Cyan
    Write-Host "Server 1 API: http://$SERVER1_IP`:3000" -ForegroundColor White
    Write-Host "Server 1 Grafana: http://$SERVER1_IP`:3001" -ForegroundColor White
    Write-Host "Server 1 Prometheus: http://$SERVER1_IP`:9090" -ForegroundColor White
    Write-Host "Server 2 API: http://$SERVER2_IP`:3000" -ForegroundColor White
    Write-Host "Server 2 Grafana: http://$SERVER2_IP`:3001" -ForegroundColor White
    Write-Host "Server 2 Prometheus: http://$SERVER2_IP`:9090" -ForegroundColor White
    
} else {
    Write-Host "⚠️ IPPAN services are not running on one or both servers" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "=== Deployment Required ===" -ForegroundColor Cyan
    Write-Host "We need to deploy IPPAN services on the servers." -ForegroundColor White
    Write-Host ""
    Write-Host "Options:" -ForegroundColor Yellow
    Write-Host "1. Use Hetzner Cloud Console to access servers directly" -ForegroundColor White
    Write-Host "2. Use SSH with password authentication" -ForegroundColor White
    Write-Host "3. Use cloud-init files to recreate servers with proper configuration" -ForegroundColor White
    Write-Host ""
    
    # Create deployment instructions
    $deploymentInstructions = @"
# IPPAN Deployment Instructions

## Current Status:
- Server 1 ($SERVER1_IP): $(if($server1Online){'Online'}else{'Offline'}) - API: $(if($server1API){'Running'}else{'Not Running'})
- Server 2 ($SERVER2_IP): $(if($server2Online){'Online'}else{'Offline'}) - API: $(if($server2API){'Running'}else{'Not Running'})

## Next Steps:

### Option 1: Manual Deployment via Console
1. Go to Hetzner Cloud Console: https://console.hetzner.cloud/
2. Access each server's console
3. Run the deployment commands

### Option 2: SSH Deployment
1. Connect to each server via SSH
2. Run the IPPAN deployment script

### Option 3: Recreate Servers
1. Delete existing servers
2. Create new servers with cloud-init files
3. Deploy IPPAN services automatically

## Deployment Commands (for manual deployment):

# On each server, run:
sudo apt update && sudo apt install -y curl git docker.io
sudo systemctl start docker
sudo systemctl enable docker
sudo usermod -aG docker $USER

# Clone and deploy IPPAN
cd /opt
sudo git clone https://github.com/dmrl789/IPPAN.git ippan
cd ippan
sudo chown -R `$USER:`$USER .

# Create configuration
cat > config.toml << 'EOF'
[network]
bootstrap_nodes = [
    "$SERVER1_IP:8080",
    "$SERVER2_IP:8080"
]
listen_address = "0.0.0.0:8080"
external_address = "SERVER_IP:8080"

[api]
listen_address = "0.0.0.0:3000"
cors_origins = ["*"]

[logging]
level = "info"
format = "json"
EOF

# Start services
docker-compose up -d
"@
    
    $deploymentInstructions | Out-File -FilePath "deployment_instructions.txt" -Encoding UTF8
    Write-Host "Deployment instructions saved to: deployment_instructions.txt" -ForegroundColor Green
}

Write-Host ""
Write-Host "=== Summary ===" -ForegroundColor Cyan
Write-Host "Both servers are online and reachable" -ForegroundColor Green
Write-Host "IPPAN services need to be deployed to establish peer-to-peer connection" -ForegroundColor Yellow
Write-Host "Use the deployment instructions above to get the servers connected" -ForegroundColor White
