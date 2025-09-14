# IPPAN Multi-Node Deployment Guide

This guide will help you deploy IPPAN on two servers and connect them in a multi-node network.

## Server Configuration

- **Server 1 (Nuremberg)**: `188.245.97.41`
- **Server 2 (Helsinki)**: `135.181.145.174`

## Prerequisites

1. Both servers should be running Ubuntu/Debian
2. SSH access configured for the `ippan` user
3. Docker and Docker Compose installed on both servers
4. Firewall rules configured to allow communication between servers

## Deployment Steps

### Step 1: Deploy Server 1 (Nuremberg)

```bash
# Run the Server 1 deployment script
./scripts/deploy_server1.sh
```

This script will:
- Clone the IPPAN repository
- Configure the node for multi-node setup
- Create Docker Compose configuration
- Deploy IPPAN services
- Set up monitoring

### Step 2: Deploy Server 2 (Helsinki)

```bash
# Run the Server 2 deployment script
./scripts/deploy_server2_connect.sh
```

This script will:
- Clone the IPPAN repository
- Configure the node to connect to Server 1
- Create Docker Compose configuration
- Deploy IPPAN services
- Set up monitoring

### Step 3: Verify Multi-Node Setup

```bash
# Run the verification script
./scripts/verify_multi_node_deployment.sh
```

Or on Windows:
```powershell
powershell -ExecutionPolicy Bypass -File scripts/verify_multi_node_deployment.ps1
```

## Quick Connectivity Test

To quickly test if the servers are accessible:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/quick_connectivity_test.ps1
```

## Expected Results

After successful deployment, you should see:

### Server 1 (Nuremberg)
- ✅ P2P port (8080) accessible
- ✅ API port (3000) accessible
- ✅ API responding to health checks
- ✅ Prometheus metrics (9090)
- ✅ Grafana dashboard (3001)

### Server 2 (Helsinki)
- ✅ P2P port (8080) accessible
- ✅ API port (3000) accessible
- ✅ API responding to health checks
- ✅ Prometheus metrics (9090)
- ✅ Grafana dashboard (3001)

### Inter-Node Connectivity
- ✅ Server 1 can reach Server 2 P2P
- ✅ Server 2 can reach Server 1 P2P
- ✅ Both nodes participating in consensus

## Access URLs

### Server 1 (Nuremberg)
- API: http://188.245.97.41:3000
- Grafana: http://188.245.97.41:3001
- Prometheus: http://188.245.97.41:9090

### Server 2 (Helsinki)
- API: http://135.181.145.174:3000
- Grafana: http://135.181.145.174:3001
- Prometheus: http://135.181.145.174:9090

## Troubleshooting

### If servers are not accessible:

1. **Check SSH connectivity**:
   ```bash
   ssh ippan@188.245.97.41
   ssh ippan@135.181.145.174
   ```

2. **Check firewall rules**:
   ```bash
   # On each server
   sudo ufw status
   ```

3. **Check Docker services**:
   ```bash
   # On each server
   docker ps
   docker-compose ps
   ```

4. **Check logs**:
   ```bash
   # On each server
   docker-compose logs -f
   ```

### If nodes are not connecting:

1. **Check network configuration**:
   - Ensure both servers can reach each other on port 8080
   - Verify bootstrap nodes are correctly configured

2. **Check consensus participation**:
   ```bash
   curl http://188.245.97.41:3000/api/v1/blockchain/latest
   curl http://135.181.145.174:3000/api/v1/blockchain/latest
   ```

3. **Check peer connections**:
   ```bash
   curl http://188.245.97.41:3000/api/v1/network/peers
   curl http://135.181.145.174:3000/api/v1/network/peers
   ```

## Monitoring

Both servers include:
- **Prometheus**: Metrics collection
- **Grafana**: Dashboards and visualization
- **Health checks**: API endpoints for monitoring

## Security Notes

- SSH access is restricted to specific IPs
- Firewall rules are configured for IPPAN services
- Fail2ban is enabled for SSH protection
- Services run in Docker containers for isolation

## Next Steps

After successful deployment:
1. Monitor the network health via Grafana dashboards
2. Test transaction processing between nodes
3. Set up automated backups
4. Configure alerting for critical events
5. Scale the network by adding more nodes if needed
