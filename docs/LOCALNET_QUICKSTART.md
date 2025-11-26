# Localnet Quickstart Guide

Get IPPAN running locally on Windows in under 10 minutes.

## Prerequisites

- **Git** - [Download](https://git-scm.com/download/win)
- **Rust toolchain** - Install via [rustup](https://rustup.rs/)
- **Docker Desktop** - [Download](https://www.docker.com/products/docker-desktop/)
  - Enable "Use WSL2 based engine" in Docker Desktop settings
  - Ensure Docker Desktop is running before proceeding

## One-Command Start

Open PowerShell in the repository root and run:

```powershell
.\localnet\run.ps1
```

The script will:
1. Verify Docker and Docker Compose are available
2. Validate the compose configuration
3. Build container images (first run only, ~5-10 minutes)
4. Start all services in detached mode
5. Display service status and endpoints

## Verify It's Running

### Check Service Status

```powershell
docker compose -f localnet/docker-compose.full-stack.yaml -p ippan-local ps
```

All services should show `Up` status.

### View Logs

```powershell
docker compose -f localnet/docker-compose.full-stack.yaml -p ippan-local logs -f
```

Press `Ctrl+C` to stop following logs.

### Test Health Endpoint

```powershell
Invoke-WebRequest -Uri http://localhost:8080/health -UseBasicParsing
```

Or using curl (if available):

```powershell
curl http://localhost:8080/health
```

Expected response: JSON with `status: "ok"` and node information.

### Access the UI

Open your browser to:
- **Unified Explorer UI**: http://localhost:3000
- **Gateway API**: http://localhost:8081/api
- **Node RPC**: http://localhost:8080

## Stop Localnet

```powershell
.\localnet\stop.ps1
```

This stops all services and removes containers/volumes. Data in `localnet/data/` persists unless you manually delete it.

## Services & Ports

| Service | URL | Description |
|---------|-----|-------------|
| Node RPC | `http://localhost:8080` | Native RPC + health endpoints |
| Node P2P | `localhost:9000` | P2P gossip (devnet only) |
| Gateway API | `http://localhost:8081/api` | REST/WebSocket proxy |
| Gateway WS | `ws://localhost:8081/ws` | Real-time WebSocket stream |
| Unified UI | `http://localhost:3000` | Next.js explorer interface |

## Troubleshooting

### Docker Not Running

**Error**: `Docker is not available or not running`

**Fix**: 
1. Start Docker Desktop
2. Wait for Docker to fully start (whale icon in system tray)
3. Ensure "Use WSL2 based engine" is enabled in Settings → General
4. Re-run `.\localnet\run.ps1`

### Port Already in Use

**Error**: `Bind for 0.0.0.0:8080 failed: port is already allocated`

**Fix**:
1. Stop the conflicting service:
   ```powershell
   .\localnet\stop.ps1
   ```
2. Or find and stop the process using the port:
   ```powershell
   netstat -ano | findstr :8080
   # Note the PID, then:
   taskkill /PID <PID> /F
   ```

### Windows Firewall

If you can't access endpoints, Windows Firewall may be blocking. Allow Docker Desktop through the firewall:
1. Windows Security → Firewall & network protection
2. Allow an app through firewall
3. Find "Docker Desktop" and enable both Private and Public networks

### Clean Reset

To start completely fresh (removes all data):

```powershell
.\localnet\stop.ps1
Remove-Item -Recurse -Force localnet/data -ErrorAction SilentlyContinue
.\localnet\run.ps1
```

### Containers Stuck on "Starting"

Check logs for errors:

```powershell
docker compose -f localnet/docker-compose.full-stack.yaml -p ippan-local logs
```

Common issues:
- **Build failures**: Ensure Docker has enough resources (Settings → Resources → Memory ≥ 4GB)
- **Image pull errors**: Check internet connection and Docker registry access
- **Permission errors**: Run PowerShell as Administrator (rarely needed)

## Export Training Dataset

You can export validator metrics from the running localnet to generate training data for the GBDT model:

1. **Ensure localnet is running**: `.\localnet\run.ps1`

2. **Export dataset**:
   ```powershell
   .\localnet\export-dataset.ps1
   ```

   This script:
   - Fetches validator metrics from the RPC endpoint (`http://localhost:8080/status`)
   - Collects 120 samples at 5-second intervals (default)
   - Exports to `ai_training/localnet_training.csv` (gitignored)

3. **Train the model v2**:
   ```powershell
   python ai_training\train_ippan_d_gbdt.py --csv ai_training/localnet_training.csv --out ai_training/ippan_d_gbdt_v2.json
   ```

4. **Compute model hash and promote to runtime**:
   ```powershell
   # Compute BLAKE3 hash
   python -c "from blake3 import blake3; p='ai_training/ippan_d_gbdt_v2.json'; print(blake3(open(p,'rb').read()).hexdigest())"
   
   # Copy to runtime location
   Copy-Item ai_training\ippan_d_gbdt_v2.json crates\ai_registry\models\ippan_d_gbdt_v2.json
   
   # Update config/dlc.toml with the hash
   # Update ai_training/model_card_ippan_d_gbdt_v2.toml with the hash
   ```

**Note**: The exported features are "proxy 7d" approximations (windowed deltas from current metrics) suitable for bootstrap/testing. For production training, use longer collection periods or aggregate historical data. The v2 model is vendored under `crates/ai_registry/models/` and enforced via strict hash verification at startup.

## Next Steps

- **Send a test transaction**: See [Local Full-Stack Guide](./dev/local-full-stack.md) for end-to-end examples
- **Explore the API**: Visit http://localhost:8081/api/docs (if available) or check the [API documentation](../README.md#-api-endpoints)
- **View blocks/transactions**: Use the explorer UI at http://localhost:3000
- **Developer workflow**: See [Developer Journey](./dev/developer-journey.md)

## Related Documentation

- [Local Full-Stack Guide](./dev/local-full-stack.md) - Detailed setup and usage
- [Developer Journey](./dev/developer-journey.md) - Complete developer onboarding
- [Node Operator Guide](./operators/NODE_OPERATOR_GUIDE.md) - Production deployment
