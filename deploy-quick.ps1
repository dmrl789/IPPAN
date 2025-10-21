# Quick IPPAN Deployment Script
# This script deploys the unified UI and fixes node connectivity

param(
    [string]$ServerIP = "188.245.97.41",
    [string]$UIDomain = "188.245.97.41:3001"
)

Write-Host "🚀 IPPAN Quick Deployment" -ForegroundColor Blue
Write-Host "========================="

# Check if deployment files exist
if (-not (Test-Path "deploy-unified-ui.yml")) {
    Write-Host "❌ deploy-unified-ui.yml not found. Run fix-deployment.ps1 first." -ForegroundColor Red
    exit 1
}

if (-not (Test-Path "nginx.conf")) {
    Write-Host "❌ nginx.conf not found. Run fix-deployment.ps1 first." -ForegroundColor Red
    exit 1
}

Write-Host "✅ Deployment files found" -ForegroundColor Green

# Test server connectivity
Write-Host "🔍 Testing server connectivity..." -ForegroundColor Yellow
$serverReachable = Test-Connection -ComputerName $ServerIP -Count 1 -Quiet

if (-not $serverReachable) {
    Write-Host "❌ Server $ServerIP is not reachable" -ForegroundColor Red
    Write-Host "Please check the server status and try again." -ForegroundColor Yellow
    exit 1
}

Write-Host "✅ Server $ServerIP is reachable" -ForegroundColor Green

# Create deployment package
Write-Host "📦 Creating deployment package..." -ForegroundColor Yellow

# Create a simple deployment script for the server
$deployScript = @"
#!/bin/bash
set -euo pipefail

echo "🚀 Deploying IPPAN Unified UI..."

# Stop any existing containers
docker-compose down 2>/dev/null || true

# Free up ports
echo "Freeing up ports..."
lsof -ti:80,443,8080,8081,9000,3001 | xargs -r kill -9 2>/dev/null || true

# Pull latest images
echo "Pulling latest images..."
docker-compose pull

# Start services
echo "Starting services..."
docker-compose up -d

# Wait for services to start
echo "Waiting for services to start..."
sleep 15

# Check status
echo "Checking service status..."
docker-compose ps

# Test endpoints
echo "Testing endpoints..."
curl -s http://localhost:8080/health || echo "Node health check failed"
curl -s http://localhost:8081/health || echo "Gateway health check failed"
curl -s http://localhost:3001 || echo "UI health check failed"

echo "✅ Deployment completed!"
echo "UI should be available at: http://$ServerIP"
echo "API should be available at: http://$ServerIP:8081"
"@

$deployScript | Out-File -FilePath "deploy-on-server.sh" -Encoding UTF8

Write-Host "✅ Deployment package created" -ForegroundColor Green

# Display next steps
Write-Host ""
Write-Host "📋 Next Steps:" -ForegroundColor Blue
Write-Host "==============="
Write-Host ""
Write-Host "1. Upload files to server:" -ForegroundColor Yellow
Write-Host "   scp deploy-unified-ui.yml root@$ServerIP`:/root/"
Write-Host "   scp nginx.conf root@$ServerIP`:/root/"
Write-Host "   scp deploy-on-server.sh root@$ServerIP`:/root/"
Write-Host ""
Write-Host "2. SSH to server and deploy:" -ForegroundColor Yellow
Write-Host "   ssh root@$ServerIP"
Write-Host "   chmod +x deploy-on-server.sh"
Write-Host "   ./deploy-on-server.sh"
Write-Host ""
Write-Host "3. Configure DNS:" -ForegroundColor Yellow
Write-Host "   Point $UIDomain to $ServerIP"
Write-Host ""
Write-Host "4. Test deployment:" -ForegroundColor Yellow
Write-Host "   curl -I http://$ServerIP"
Write-Host "   curl http://$ServerIP`:8081/health"
Write-Host ""

# Test current server status
Write-Host "🔍 Current Server Status:" -ForegroundColor Blue
Write-Host "========================="

# Test RPC port
$rpcPort = Test-NetConnection -ComputerName $ServerIP -Port 8080 -InformationLevel Quiet
if ($rpcPort) {
    Write-Host "✅ RPC API (port 8080) is accessible" -ForegroundColor Green
} else {
    Write-Host "❌ RPC API (port 8080) is not accessible" -ForegroundColor Red
}

# Test P2P port
$p2pPort = Test-NetConnection -ComputerName $ServerIP -Port 9000 -InformationLevel Quiet
if ($p2pPort) {
    Write-Host "✅ P2P (port 9000) is accessible" -ForegroundColor Green
} else {
    Write-Host "❌ P2P (port 9000) is not accessible" -ForegroundColor Red
}

# Test HTTP port
$httpPort = Test-NetConnection -ComputerName $ServerIP -Port 80 -InformationLevel Quiet
if ($httpPort) {
    Write-Host "✅ HTTP (port 80) is accessible" -ForegroundColor Green
} else {
    Write-Host "❌ HTTP (port 80) is not accessible" -ForegroundColor Red
}

Write-Host ""
Write-Host "🎯 Ready to deploy!" -ForegroundColor Green
Write-Host "Run the commands above to deploy the unified UI." -ForegroundColor Yellow
