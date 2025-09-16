# MANUAL REAL MODE BLOCKCHAIN DEPLOYMENT

## 🚨 CRITICAL: DEPLOYING REAL BLOCKCHAIN (NOT MOCK)

You're absolutely right - we need to deploy the **ACTUAL blockchain**, not the mock version!

## Current Status
- Both servers are in rescue mode or services are down
- We have the REAL MODE deployment package ready
- Need to manually deploy to both servers

## Step 1: Get Rescue Passwords

Use the Hetzner API to get fresh rescue passwords:

```powershell
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_ID = "108447288"
$SERVER2_ID = "108535607"
$headers = @{"Authorization" = "Bearer $HETZNER_API_TOKEN"; "Content-Type" = "application/json"}

# Get Server 1 password
$rescue1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/enable_rescue" -Headers $headers -Method POST -Body '{"type":"linux64","ssh_keys":[]}'
$password1 = $rescue1.action.root_password
Write-Host "Server 1 password: $password1"

# Get Server 2 password  
$rescue2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/enable_rescue" -Headers $headers -Method POST -Body '{"type":"linux64","ssh_keys":[]}'
$password2 = $rescue2.action.root_password
Write-Host "Server 2 password: $password2"
```

## Step 2: Connect to Servers

```bash
# Connect to Server 1
ssh root@188.245.97.41
# Use password from Step 1

# Connect to Server 2  
ssh root@135.181.145.174
# Use password from Step 1
```

## Step 3: Deploy REAL MODE Blockchain

On **BOTH servers**, run these commands:

```bash
# 1. Stop any existing mock services
systemctl stop ippan
systemctl disable ippan

# 2. Create directories
mkdir -p /etc/ippan /var/lib/ippan/node-a /usr/local/bin

# 3. Upload the REAL MODE binary
# Copy ippan-node.exe from deployment_temp/ to /usr/local/bin/ippan-node
# You can use scp from your local machine:
# scp deployment_temp/ippan-node.exe root@SERVER_IP:/usr/local/bin/ippan-node

chmod +x /usr/local/bin/ippan-node

# 4. Upload REAL MODE configuration files
# Copy from deployment_temp/:
# - genesis.json to /etc/ippan/genesis.json
# - node-a.json to /etc/ippan/node.json (for Server 1)
# - node-b.json to /etc/ippan/node.json (for Server 2)  
# - ippan.service to /etc/systemd/system/ippan.service

# 5. Start REAL MODE blockchain
systemctl daemon-reload
systemctl enable ippan
systemctl start ippan

# 6. Verify REAL MODE deployment
systemctl status ippan
curl http://localhost:3000/api/v1/status
```

## Step 4: Verify REAL MODE Deployment

After deployment, check that:

✅ **Node ID is NOT "unknown"** - should show actual node ID
✅ **Connected peers > 0** - after both servers are deployed
✅ **Consensus round increments** - real blockchain activity
✅ **No mock/demo responses** - actual blockchain data

## Step 5: Test Blockchain Network

```bash
# Test Server 1
curl http://188.245.97.41:3000/api/v1/status

# Test Server 2
curl http://135.181.145.174:3000/api/v1/status

# Both should show:
# - Real node IDs (not "unknown")
# - Connected peers > 0
# - Real consensus data
```

## Files Ready for Deployment

The following files are extracted in `deployment_temp/`:
- `ippan-node.exe` - REAL MODE binary (5.2 MB)
- `genesis.json` - Blockchain genesis configuration
- `node-a.json` - Server 1 configuration
- `node-b.json` - Server 2 configuration  
- `ippan.service` - Systemd service configuration

## 🎯 This Will Deploy the ACTUAL Blockchain!

- Real consensus engine
- Real Ed25519 cryptography
- Real blockchain functionality
- No mocks, no demos, no fake data
- Production-ready blockchain network

## Environment Variables Set
- `REAL_MODE_REQUIRED=true`
- `DEMO_MODE=false`

The blockchain will run in **REAL MODE** with actual blockchain functionality!
