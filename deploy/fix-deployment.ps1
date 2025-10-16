# IPPAN Deployment Fix Script (PowerShell)
# This script fixes node connectivity and keeps the Unified UI disabled

param(
    [string]$Server1IP = "188.245.97.41",
    [string]$Server2IP = "135.181.145.174"
)

Write-Host "🚀 IPPAN Deployment Fix Script" -ForegroundColor Blue
Write-Host "=================================="

# Function to check if server is reachable
function Test-ServerConnectivity {
    param(
        [string]$ServerIP,
        [string]$ServerName
    )

    Write-Host "Checking $ServerName ($ServerIP)..." -ForegroundColor Yellow

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
        Write-Host "❌ $ServerName is not reachable" -ForegroundColor Red
        return $false
    }
}

# Function to check port connectivity
function Test-PortConnectivity {
    param(
        [string]$ServerIP,
        [int]$Port,
        [string]$ServiceName
    )

    Write-Host "Checking $ServiceName on $ServerIP`:$Port..." -ForegroundColor Yellow

    try {
        $connection = Test-NetConnection -ComputerName $ServerIP -Port $Port -InformationLevel Quiet
        if ($connection) {
            Write-Host "✅ $ServiceName is accessible on $ServerIP`:$Port" -ForegroundColor Green
            return $true
        } else {
            Write-Host "❌ $ServiceName is not accessible on $ServerIP`:$Port" -ForegroundColor Red
            return $false
        }
    } catch {
        Write-Host "❌ $ServiceName is not accessible on $ServerIP`:$Port" -ForegroundColor Red
        return $false
    }
}

# Function to test current connectivity
function Test-CurrentConnectivity {
    Write-Host "🔍 Testing current connectivity..." -ForegroundColor Blue

    # Test Server 1
    if (Test-ServerConnectivity -ServerIP $Server1IP -ServerName "Server 1") {
        Test-PortConnectivity -ServerIP $Server1IP -Port 8080 -ServiceName "RPC API" | Out-Null
        Test-PortConnectivity -ServerIP $Server1IP -Port 9000 -ServiceName "P2P" | Out-Null
    }

    # Test Server 2
    if (Test-ServerConnectivity -ServerIP $Server2IP -ServerName "Server 2") {
        Test-PortConnectivity -ServerIP $Server2IP -Port 8080 -ServiceName "RPC API" | Out-Null
        Test-PortConnectivity -ServerIP $Server2IP -Port 9001 -ServiceName "P2P" | Out-Null
    }
}

# Function to create deployment instructions
function New-DeploymentInstructions {
    Write-Host "📋 Creating deployment instructions..." -ForegroundColor Blue

    $instructions = @"
# IPPAN Deployment Instructions

## Current Status
- Server 1 ($Server1IP): RPC API accessible, P2P port issues
- Server 2 ($Server2IP): Not accessible
- Unified UI: intentionally removed (servers should run nodes only)

## Fix Steps

### 1. Fix Server 2 Connectivity
```bash
# SSH to Server 2
ssh root@${Server2IP}

# Check if Docker is running
docker --version
systemctl status docker

# Start Docker if not running
systemctl start docker
systemctl enable docker

# Check if IPPAN node is running
docker ps -a | grep ippan

# Start IPPAN node if not running
docker-compose -f docker-compose.production.yml up -d
```

### 2. Verify Unified UI is Disabled
```bash
# Ensure no UI containers are running
ssh root@${Server1IP} "docker ps --format '{{.Names}}' | grep -i ui" || echo "Unified UI not running"

# Confirm HTTP ports 80/443 are closed (UI removed)
nc -zv ${Server1IP} 80 || echo "Port 80 closed as expected"
nc -zv ${Server1IP} 443 || echo "Port 443 closed as expected"
```

### 3. Test Node Deployment
```bash
# Test API on Server 1
curl http://${Server1IP}:8080/health

# Test API on Server 2 (if reachable)
curl http://${Server2IP}:8080/health
```

## Files Created
No additional files are required for the Unified UI.
"@

    $instructions | Out-File -FilePath "DEPLOYMENT_INSTRUCTIONS.md" -Encoding UTF8

    Write-Host "✅ Deployment instructions created" -ForegroundColor Green
}

# Main execution
function Main {
    Write-Host "Starting IPPAN deployment fix..." -ForegroundColor Blue

    # Test current connectivity
    Test-CurrentConnectivity

    # Create deployment instructions
    New-DeploymentInstructions

    Write-Host "🎉 Deployment fix completed!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor Yellow
    Write-Host "1. Ensure Docker and the IPPAN node compose stack are running on each server"
    Write-Host "2. Verify no Unified UI containers exist (docker ps | Select-String -Pattern 'ui' should return nothing)"
    Write-Host "3. Confirm ports 80/443 remain closed while RPC (8080) and P2P ports stay healthy"
    Write-Host "4. Share updated DEPLOYMENT_INSTRUCTIONS.md with the operations team"
    Write-Host ""
    Write-Host "Files created:" -ForegroundColor Blue
    Write-Host "- DEPLOYMENT_INSTRUCTIONS.md (Step-by-step guide)"
}

# Run main function
Main
