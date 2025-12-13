# Hetzner Devnet-1 Deployment

Quick reference for deploying IPPAN devnet on Hetzner Cloud.

## Prerequisites

✅ **Readiness Pulse:** GREEN (Run ID: 20001036008)  
✅ **Audit Pack Evidence:** Confirmed  
✅ **Decision:** GO for Devnet-1

## Quick Start

### 1. Provision 4 Hetzner Servers
- Ubuntu 24.04 LTS
- 4 vCPU / 8GB RAM / 80GB disk minimum
- Private network enabled (recommended)

### 2. Run Setup Script on Each Node

**Node 1 (Bootstrap):**
```bash
./deploy/hetzner/scripts/setup-node.sh node1
```

**Node 2, 3, 4 (replace NODE1_IP with node1's private IP):**
```bash
./deploy/hetzner/scripts/setup-node.sh node2 NODE1_IP
./deploy/hetzner/scripts/setup-node.sh node3 NODE1_IP
./deploy/hetzner/scripts/setup-node.sh node4 NODE1_IP
```

### 3. Start Services

On each node:
```bash
sudo systemctl start ippan-node
sudo systemctl status ippan-node
```

### 4. Validate

From your laptop:
```bash
# Replace NODE4_IP with node4's public IP
curl http://NODE4_IP:8080/status | jq .
```

## Files

- `docs/operators/HETZNER_DEVNET_DEPLOYMENT.md` - Full deployment guide
- `deploy/hetzner/systemd/ippan-node.service` - Systemd service file
- `deploy/hetzner/scripts/setup-node.sh` - Automated setup script

## Ports

- **RPC/HTTP:** 8080
- **P2P:** 9000 (TCP/UDP)

## Troubleshooting

See full guide: `docs/operators/HETZNER_DEVNET_DEPLOYMENT.md`

