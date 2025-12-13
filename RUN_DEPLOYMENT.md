# IPPAN Devnet-1 Deployment - Ready to Execute

## Server Configuration
```powershell
$NODE1_PUB="188.245.97.41"
$NODE1_PRIV="10.0.0.2"
$NODE2_PUB="135.181.145.174"
$NODE2_PRIV="10.0.0.3"
$NODE3_PUB="5.223.51.238"
$NODE4_PUB="178.156.219.107"
$SERVERS=@($NODE1_PUB,$NODE2_PUB,$NODE3_PUB,$NODE4_PUB)
```

---

## PHASE 2: Copy Repository to Each Server

### Setup directories
```powershell
foreach($S in $SERVERS){
  ssh ippan@$S "sudo mkdir -p /opt/ippan && sudo chown -R ippan:ippan /opt/ippan"
}
```

### Clone/pull repository
```powershell
foreach($S in $SERVERS){
  ssh ippan@$S "cd /opt/ippan && (test -d .git && git fetch origin && git reset --hard origin/master || git clone https://github.com/dmrl789/IPPAN .) && git checkout master && git pull --ff-only"
}
```

### Make setup script executable
```powershell
foreach($S in $SERVERS){
  ssh ippan@$S "cd /opt/ippan && chmod +x deploy/hetzner/scripts/setup-node.sh || true"
}
```

---

## PHASE 3: Run Setup Scripts

```powershell
# node1 (bootstrap) — no bootstrap arg
ssh ippan@$NODE1_PUB "cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh node1"

# node2 — bootstrap via PRIVATE IP (better)
ssh ippan@$NODE2_PUB "cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh node2 $NODE1_PRIV"

# node3 — bootstrap via PUBLIC IP (not on private net)
ssh ippan@$NODE3_PUB "cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh node3 $NODE1_PUB"

# node4 (observer/rpc) — bootstrap via PUBLIC IP
ssh ippan@$NODE4_PUB "cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh node4 $NODE1_PUB"
```

**Note:** Each setup script will take 15-30 minutes to build the binary.

---

## PHASE 4: Start Services + Verify Ports

```powershell
# Start + enable on all nodes
foreach($S in $SERVERS){
  ssh ippan@$S "sudo systemctl daemon-reload && sudo systemctl enable ippan-node && sudo systemctl restart ippan-node && sleep 2 && sudo systemctl --no-pager --full status ippan-node | head -n 30"
}

# Quick port checks (RPC on node4)
ssh ippan@$NODE4_PUB "ss -lntup | egrep ':(8080|9000)\b' || true"
```

---

## PHASE 5: Bootstrap Connectivity Checks

```powershell
# Check logs for peer connect messages (tail 80 lines)
ssh ippan@$NODE1_PUB "sudo tail -n 80 /var/log/ippan/node.log || sudo journalctl -u ippan-node -n 80 --no-pager"
ssh ippan@$NODE2_PUB "sudo tail -n 80 /var/log/ippan/node.log || sudo journalctl -u ippan-node -n 80 --no-pager"
ssh ippan@$NODE3_PUB "sudo tail -n 80 /var/log/ippan/node.log || sudo journalctl -u ippan-node -n 80 --no-pager"
ssh ippan@$NODE4_PUB "sudo tail -n 80 /var/log/ippan/node.log || sudo journalctl -u ippan-node -n 80 --no-pager"

# Query /status on node4 (from laptop)
curl http://178.156.219.107:8080/status
```

---

## PHASE 6: Fix Firewall on Node1 (if needed)

```powershell
# Node1 must accept inbound P2P from node3/node4 public internet
ssh ippan@$NODE1_PUB "sudo ufw allow 9000/tcp; sudo ufw allow 9000/udp; sudo ufw allow 8080/tcp; sudo ufw status numbered"

# Restart node1 after firewall change
ssh ippan@$NODE1_PUB "sudo systemctl restart ippan-node"

# Re-check status
curl http://178.156.219.107:8080/status
```

---

## PHASE 7: Validation Checklist

```powershell
# On node4: metrics available + validators >= 3
curl http://178.156.219.107:8080/status

# Verify no crash loops
foreach($S in $SERVERS){
  ssh ippan@$S "sudo systemctl is-active ippan-node && echo OK || (echo FAIL; sudo journalctl -u ippan-node -n 120 --no-pager)"
}
```

---

## Expected Results

**STOP when:**
- ✅ All 4 services are active
- ✅ node4 /status responds reliably  
- ✅ Validators count shows expected (>=3) OR peers list shows all nodes connected
- ✅ No crash loops in journald/log files

---

## Report Back

After running the deployment, please provide:

1. **Status endpoint output:**
   ```powershell
   curl http://178.156.219.107:8080/status
   ```

2. **If node3 fails, provide logs:**
   ```powershell
   ssh ippan@5.223.51.238 "sudo journalctl -u ippan-node -n 30 --no-pager"
   ```

