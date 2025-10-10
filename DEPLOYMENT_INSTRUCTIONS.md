# IPPAN Deployment Instructions

## Current Status
- Server 1 (188.245.97.41): RPC API accessible, P2P port issues
- Server 2 (135.181.145.174): Not accessible
- Unified UI: Not deployed

## Fix Steps

### 1. Fix Server 2 Connectivity
`ash
# SSH to Server 2
ssh root@135.181.145.174

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
`

### 2. Deploy Unified UI to Server 1
`ash
# SSH to Server 1
ssh root@188.245.97.41

# Upload deployment files
scp deploy-unified-ui.yml root@188.245.97.41:/root/
scp nginx.conf root@188.245.97.41:/root/

# Deploy services
cd /root
docker-compose -f deploy-unified-ui.yml up -d

# Check status
docker-compose -f deploy-unified-ui.yml ps
`

### 3. Configure DNS
Point ui.ippan.org to 188.245.97.41

### 4. Test Deployment
`ash
# Test UI
curl -I http://188.245.97.41
curl -I https://ui.ippan.org

# Test API
curl http://188.245.97.41:8081/health
curl https://ui.ippan.org/api/health
`

## Files Created
- deploy-unified-ui.yml (Docker Compose configuration)
- nginx.conf (Nginx reverse proxy configuration)
