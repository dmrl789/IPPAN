# IPPAN Deployment Fix Script (PowerShell)
# This script fixes node connectivity and deploys unified UI

param(
    [string]$Server1IP = "188.245.97.41",
    [string]$Server2IP = "135.181.145.174",
    [string]$UIDomain = "ui.ippan.org"
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

# Function to create deployment files
function New-DeploymentFiles {
    Write-Host "🌐 Creating deployment files..." -ForegroundColor Blue
    
    # Create Docker Compose file
    $dockerComposeContent = @"
version: '3.8'

services:
  # IPPAN Blockchain Node
  ippan-node:
    image: ghcr.io/dmrl789/ippan/ippan-node:latest
    container_name: ippan-node
    environment:
      - NODE_ID=ippan_production_node_001
      - VALIDATOR_ID=0000000000000000000000000000000000000000000000000000000000000001
      - RPC_HOST=0.0.0.0
      - RPC_PORT=8080
      - P2P_HOST=0.0.0.0
      - P2P_PORT=9000
      - P2P_BOOTNODES=http://$Server2IP:9001
      - STORAGE_PATH=/var/lib/ippan/db
      - LOG_LEVEL=info
      - LOG_FORMAT=json
    ports:
      - "8080:8080"  # RPC API
      - "9000:9000"  # P2P
    volumes:
      - node_data:/var/lib/ippan/db
    restart: unless-stopped
    networks:
      - ippan-network

  # Unified UI
  unified-ui:
    image: ghcr.io/dmrl789/ippan/unified-ui:latest
    container_name: unified-ui
    environment:
      - NODE_ENV=production
      - PORT=3000
      - NEXT_PUBLIC_GATEWAY_URL=https://$UIDomain/api
      - NEXT_PUBLIC_API_BASE_URL=https://$UIDomain/api
      - NEXT_PUBLIC_WS_URL=wss://$UIDomain/ws
      - NEXT_PUBLIC_ENABLE_FULL_UI=1
      - NEXT_PUBLIC_NETWORK_NAME=IPPAN-Devnet
    ports:
      - "3001:3000"  # UI
    restart: unless-stopped
    depends_on:
      - ippan-node
    networks:
      - ippan-network

  # API Gateway
  gateway:
    image: ghcr.io/dmrl789/ippan/gateway:latest
    container_name: gateway
    environment:
      - GATEWAY_HOST=0.0.0.0
      - GATEWAY_PORT=8081
      - NODE_RPC_URL=http://ippan-node:8080
      - GATEWAY_ALLOWED_ORIGINS=https://$UIDomain
    ports:
      - "8081:8081"  # Gateway API
    restart: unless-stopped
    depends_on:
      - ippan-node
    networks:
      - ippan-network

  # Nginx Reverse Proxy
  nginx:
    image: nginx:alpine
    container_name: nginx-proxy
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
    restart: unless-stopped
    depends_on:
      - unified-ui
      - gateway
    networks:
      - ippan-network

volumes:
  node_data:

networks:
  ippan-network:
    driver: bridge
"@

    $dockerComposeContent | Out-File -FilePath "deploy-unified-ui.yml" -Encoding UTF8

    # Create Nginx configuration
    $nginxContent = @"
events {
    worker_connections 1024;
}

http {
    upstream ui {
        server unified-ui:3000;
    }
    
    upstream api {
        server gateway:8081;
    }
    
    server {
        listen 80;
        server_name $UIDomain;
        
        location / {
            proxy_pass http://ui;
            proxy_set_header Host `$host;
            proxy_set_header X-Real-IP `$remote_addr;
            proxy_set_header X-Forwarded-For `$proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto `$scheme;
        }
        
        location /api/ {
            proxy_pass http://api/;
            proxy_set_header Host `$host;
            proxy_set_header X-Real-IP `$remote_addr;
            proxy_set_header X-Forwarded-For `$proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto `$scheme;
        }
        
        location /ws {
            proxy_pass http://api/ws;
            proxy_http_version 1.1;
            proxy_set_header Upgrade `$http_upgrade;
            proxy_set_header Connection "upgrade";
            proxy_set_header Host `$host;
            proxy_set_header X-Real-IP `$remote_addr;
            proxy_set_header X-Forwarded-For `$proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto `$scheme;
        }
    }
}
"@

    $nginxContent | Out-File -FilePath "nginx.conf" -Encoding UTF8

    Write-Host "✅ Deployment files created" -ForegroundColor Green
}

# Function to test current connectivity
function Test-CurrentConnectivity {
    Write-Host "🔍 Testing current connectivity..." -ForegroundColor Blue
    
    # Test Server 1
    if (Test-ServerConnectivity -ServerIP $Server1IP -ServerName "Server 1") {
        Test-PortConnectivity -ServerIP $Server1IP -Port 8080 -ServiceName "RPC API"
        Test-PortConnectivity -ServerIP $Server1IP -Port 9000 -ServiceName "P2P"
    }
    
    # Test Server 2
    if (Test-ServerConnectivity -ServerIP $Server2IP -ServerName "Server 2") {
        Test-PortConnectivity -ServerIP $Server2IP -Port 8080 -ServiceName "RPC API"
        Test-PortConnectivity -ServerIP $Server2IP -Port 9001 -ServiceName "P2P"
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
- Unified UI: Not deployed

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

### 2. Deploy Unified UI to Server 1
```bash
# SSH to Server 1
ssh root@${Server1IP}

# Upload deployment files
scp deploy-unified-ui.yml root@${Server1IP}:/root/
scp nginx.conf root@${Server1IP}:/root/

# Deploy services
cd /root
docker-compose -f deploy-unified-ui.yml up -d

# Check status
docker-compose -f deploy-unified-ui.yml ps
```

### 3. Configure DNS
Point ${UIDomain} to ${Server1IP}

### 4. Test Deployment
```bash
# Test UI
curl -I http://${Server1IP}
curl -I https://${UIDomain}

# Test API
curl http://${Server1IP}:8081/health
curl https://${UIDomain}/api/health
```

## Files Created
- deploy-unified-ui.yml (Docker Compose configuration)
- nginx.conf (Nginx reverse proxy configuration)
"@

    $instructions | Out-File -FilePath "DEPLOYMENT_INSTRUCTIONS.md" -Encoding UTF8
    
    Write-Host "✅ Deployment instructions created" -ForegroundColor Green
}

# Main execution
function Main {
    Write-Host "Starting IPPAN deployment fix..." -ForegroundColor Blue
    
    # Test current connectivity
    Test-CurrentConnectivity
    
    # Create deployment files
    New-DeploymentFiles
    
    # Create deployment instructions
    New-DeploymentInstructions
    
    Write-Host "🎉 Deployment fix completed!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor Yellow
    Write-Host "1. Upload the deployment files to your server"
    Write-Host "2. Run: docker-compose -f deploy-unified-ui.yml up -d"
    Write-Host "3. Configure DNS to point ${UIDomain} to ${Server1IP}"
    Write-Host "4. Test the UI at http://${Server1IP}"
    Write-Host ""
    Write-Host "Files created:" -ForegroundColor Blue
    Write-Host "- deploy-unified-ui.yml (Docker Compose configuration)"
    Write-Host "- nginx.conf (Nginx reverse proxy configuration)"
    Write-Host "- DEPLOYMENT_INSTRUCTIONS.md (Step-by-step guide)"
}

# Run main function
Main
