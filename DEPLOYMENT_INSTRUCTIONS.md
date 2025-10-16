# IPPAN Deployment Instructions

## Current Status
- Server 1 (188.245.97.41): RPC API accessible, P2P port issues
- Server 2 (135.181.145.174): Not accessible
- Unified UI: intentionally removed (servers should run nodes only)

## Fix Steps

### 1. Fix Server 2 Connectivity
```bash
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
```

### 2. Verify Unified UI is Disabled
```bash
# Ensure no UI containers are running
ssh root@188.245.97.41 "docker ps --format '{{.Names}}' | grep -i ui" || echo "Unified UI not running"

# Confirm HTTP ports 80/443 are closed (UI removed)
nc -zv 188.245.97.41 80 || echo "Port 80 closed as expected"
nc -zv 188.245.97.41 443 || echo "Port 443 closed as expected"
```

### 3. Test Node Deployment
```bash
# Test API on Server 1
curl http://188.245.97.41:8080/health

# Test API on Server 2 (if reachable)
curl http://135.181.145.174:8080/health
```

### 4. GitHub Workflow Guardrails
- `.github/workflows/deploy-unified-ui.yml` now exits immediately with an error to prevent any attempt to redeploy the Unified UI from CI.
- `.github/workflows/check-nodes.yml` only probes the node API health endpoint and no longer expects a UI URL.
- Any automation or operator runbook referencing Unified UI deployment must be updated or removed to keep servers node-only.

## Files Created
No additional files are required for the Unified UI.
