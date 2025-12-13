# IPPAN Devnet-1 Hetzner Deployment - Execution Guide

## Prerequisites: SSH Key Setup

**IMPORTANT:** Before automated deployment, add your SSH public key to all servers:

Your public key:
```
ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEzegKgadJCRptuIM1aEVk/EaobuPAoMcssObcEO1uF+ ippan
```

Run these commands (enter password once for each):
```powershell
# Node 1
ssh ippan@188.245.97.41 'mkdir -p ~/.ssh && echo "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEzegKgadJCRptuIM1aEVk/EaobuPAoMcssObcEO1uF+ ippan" >> ~/.ssh/authorized_keys && chmod 700 ~/.ssh && chmod 600 ~/.ssh/authorized_keys'

# Node 2
ssh ippan@135.181.145.174 'mkdir -p ~/.ssh && echo "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEzegKgadJCRptuIM1aEVk/EaobuPAoMcssObcEO1uF+ ippan" >> ~/.ssh/authorized_keys && chmod 700 ~/.ssh && chmod 600 ~/.ssh/authorized_keys'

# Node 3
ssh ippan@5.223.51.238 'mkdir -p ~/.ssh && echo "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEzegKgadJCRptuIM1aEVk/EaobuPAoMcssObcEO1uF+ ippan" >> ~/.ssh/authorized_keys && chmod 700 ~/.ssh && chmod 600 ~/.ssh/authorized_keys'

# Node 4
ssh ippan@178.156.219.107 'mkdir -p ~/.ssh && echo "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEzegKgadJCRptuIM1aEVk/EaobuPAoMcssObcEO1uF+ ippan" >> ~/.ssh/authorized_keys && chmod 700 ~/.ssh && chmod 600 ~/.ssh/authorized_keys'
```

After this, verify passwordless SSH works:
```powershell
ssh ippan@188.245.97.41 "hostname"
```

---

## Automated Deployment (After SSH Keys Setup)

Once SSH keys are configured, run:
```powershell
.\deploy-hetzner-devnet.ps1
```

---

## Manual Deployment (If Not Using SSH Keys)

### Phase 0: Local Verification
```powershell
cd "C:\Users\yuyby\Desktop\backUp CursorSoftware\ippan nov 25 backup"
git checkout master
git pull --ff-only
git status
```

### Phase 1: Verify SSH Access
```powershell
$NODE1_PUB="188.245.97.41"; $NODE1_PRIV="10.0.0.2"
$NODE2_PUB="135.181.145.174"; $NODE2_PRIV="10.0.0.3"
$NODE3_PUB="5.223.51.238"
$NODE4_PUB="178.156.219.107"

ssh -o StrictHostKeyChecking=accept-new ippan@$NODE1_PUB "hostname && uptime"
ssh -o StrictHostKeyChecking=accept-new ippan@$NODE2_PUB "hostname && uptime"
ssh -o StrictHostKeyChecking=accept-new ippan@$NODE3_PUB "hostname && uptime"
ssh -o StrictHostKeyChecking=accept-new ippan@$NODE4_PUB "hostname && uptime"
```

### Phase 2: Copy Repo to Each Server
```powershell
$SERVERS=@($NODE1_PUB,$NODE2_PUB,$NODE3_PUB,$NODE4_PUB)

foreach($S in $SERVERS){
  ssh ippan@$S "sudo mkdir -p /opt/ippan && sudo chown -R ippan:ippan /opt/ippan"
  ssh ippan@$S "cd /opt/ippan && (test -d .git && git fetch origin && git reset --hard origin/master || git clone https://github.com/dmrl789/IPPAN .) && git checkout master && git pull --ff-only"
  ssh ippan@$S "cd /opt/ippan && chmod +x deploy/hetzner/scripts/setup-node.sh || true"
}
```

### Phase 3: Run Setup Scripts
```powershell
# node1 (bootstrap) — no bootstrap arg
ssh ippan@$NODE1_PUB "cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh node1"

# node2 — bootstrap via PRIVATE IP
ssh ippan@$NODE2_PUB "cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh node2 $NODE1_PRIV"

# node3 — bootstrap via PUBLIC IP
ssh ippan@$NODE3_PUB "cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh node3 $NODE1_PUB"

# node4 (observer/rpc) — bootstrap via PUBLIC IP
ssh ippan@$NODE4_PUB "cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh node4 $NODE1_PUB"
```

### Phase 4: Start Services
```powershell
foreach($S in $SERVERS){
  ssh ippan@$S "sudo systemctl daemon-reload && sudo systemctl enable ippan-node && sudo systemctl restart ippan-node && sleep 2 && sudo systemctl --no-pager --full status ippan-node | head -n 30"
}

# Quick port checks
ssh ippan@$NODE4_PUB "ss -lntup | egrep ':(8080|9000)\b' || true"
```

### Phase 5: Check Logs
```powershell
ssh ippan@$NODE1_PUB "sudo tail -n 80 /var/log/ippan/node.log || sudo journalctl -u ippan-node -n 80 --no-pager"
ssh ippan@$NODE2_PUB "sudo tail -n 80 /var/log/ippan/node.log || sudo journalctl -u ippan-node -n 80 --no-pager"
ssh ippan@$NODE3_PUB "sudo tail -n 80 /var/log/ippan/node.log || sudo journalctl -u ippan-node -n 80 --no-pager"
ssh ippan@$NODE4_PUB "sudo tail -n 80 /var/log/ippan/node.log || sudo journalctl -u ippan-node -n 80 --no-pager"
```

### Phase 6: Fix Firewall on Node1
```powershell
ssh ippan@$NODE1_PUB "sudo ufw allow 9000/tcp; sudo ufw allow 9000/udp; sudo ufw allow 8080/tcp; sudo ufw status numbered"
ssh ippan@$NODE1_PUB "sudo systemctl restart ippan-node"
```

### Phase 7: Final Validation
```powershell
# Check status endpoint
curl http://$NODE4_PUB:8080/status

# Verify all services are active
foreach($S in $SERVERS){
  ssh ippan@$S "sudo systemctl is-active ippan-node && echo OK || (echo FAIL; sudo journalctl -u ippan-node -n 120 --no-pager)"
}
```

---

## Expected Results

After successful deployment:

1. **All services active:**
   ```powershell
   foreach($S in $SERVERS){
     ssh ippan@$S "sudo systemctl is-active ippan-node"
   }
   ```
   Should return "active" for all 4 nodes.

2. **Status endpoint responds:**
   ```powershell
   curl http://178.156.219.107:8080/status
   ```
   Should return JSON with `status: "ok"` and `validators >= 3`.

3. **No crash loops:**
   All services should remain active without restarting.

---

## Troubleshooting

If node3/node4 can't connect to node1:
- Verify firewall on node1 allows P2P from public internet
- Check that node1 is listening on 0.0.0.0:9000 (not just private IP)
- Verify bootstrap_nodes config points to node1's public IP for node3/node4

If services fail to start:
- Check logs: `sudo journalctl -u ippan-node -n 100`
- Verify binary exists: `ls -lh /opt/ippan/target/release/ippan-node`
- Check config syntax: `/opt/ippan/target/release/ippan-node --config /etc/ippan/config/node.toml --check`

