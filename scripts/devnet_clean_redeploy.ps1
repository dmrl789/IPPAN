param(
  [string]$User = "ippan-devnet",
  [string]$Node1 = "188.245.97.41",
  [string]$Node2 = "135.181.145.174",
  [string]$Node3 = "5.223.51.238",
  [string]$Node4 = "178.156.219.107",
  [string]$RepoUrl = "https://github.com/dmrl789/IPPAN",
  [string]$Branch = "main"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Nodes = @(
  @{ name="node1"; ip=$Node1; role="node1"; },
  @{ name="node2"; ip=$Node2; role="node2"; },
  @{ name="node3"; ip=$Node3; role="node3"; },
  @{ name="node4"; ip=$Node4; role="node4"; }
)

# Bootstrap over PUBLIC IP (works for all nodes)
$BootstrapIP = $Node1

# Laptop public IP for node4 RPC allowlist
$LaptopIP = (curl.exe -fsS https://api.ipify.org).Trim()
if ([string]::IsNullOrWhiteSpace($LaptopIP)) { throw "Could not detect laptop public IP." }

$ts = Get-Date -Format "yyyyMMdd_HHmmss"
$Root = "tmp/devnet/redeploy_$ts"
New-Item -ItemType Directory -Force -Path $Root | Out-Null

function Log([string]$m) {
  $line = "[{0}] {1}" -f (Get-Date -Format "s"), $m
  $line | Tee-Object -FilePath (Join-Path $Root "run.log") -Append | Out-Host
}

function Ssh([string]$ip, [string]$cmd) {
  # Never interactive; keep output for evidence
  $out = & cmd.exe /c ("ssh -o BatchMode=yes -o ConnectTimeout=8 {0}@{1} {2}" -f $User, $ip, $cmd) 2>&1
  return $out
}

# Remote bash script: backup (optional), wipe, fresh clone, run setup-node.sh, firewall, start service
$RemoteScript = @'
set -euo pipefail

ROLE="${ROLE:?ROLE missing}"
BOOTSTRAP_IP="${BOOTSTRAP_IP:?BOOTSTRAP_IP missing}"
LAPTOP_IP="${LAPTOP_IP:?LAPTOP_IP missing}"
NODE_PUBLIC_IP="${NODE_PUBLIC_IP:?NODE_PUBLIC_IP missing}"
NODE1="${NODE1:?NODE1 missing}"
NODE2="${NODE2:?NODE2 missing}"
NODE3="${NODE3:?NODE3 missing}"
NODE4="${NODE4:?NODE4 missing}"
REPO_URL="${REPO_URL:?REPO_URL missing}"
BRANCH="${BRANCH:?BRANCH missing}"

echo "[remote] role=$ROLE public_ip=$NODE_PUBLIC_IP bootstrap_ip=$BOOTSTRAP_IP"

# sudo must be non-interactive
sudo -n true

# --- Optional backup (best effort) ---
TS="$(date -u +%Y%m%dT%H%M%SZ)"
sudo -n mkdir -p /tmp/ippan_backup || true
if [ -d /etc/ippan ] || [ -d /var/lib/ippan ] || [ -d /var/log/ippan ]; then
  sudo -n tar -czf "/tmp/ippan_backup/ippan_${ROLE}_${TS}.tgz" /etc/ippan /var/lib/ippan /var/log/ippan 2>/dev/null || true
fi

# --- STOP + WIPE (keep SSH untouched) ---
sudo -n systemctl stop ippan-node 2>/dev/null || true
sudo -n systemctl disable ippan-node 2>/dev/null || true

sudo -n rm -f /etc/systemd/system/ippan-node.service 2>/dev/null || true
sudo -n rm -rf /etc/systemd/system/ippan-node.service.d 2>/dev/null || true
sudo -n systemctl daemon-reload || true

sudo -n rm -rf /etc/ippan /var/lib/ippan /var/log/ippan /opt/ippan 2>/dev/null || true
sudo -n rm -f /usr/local/bin/ippan-node 2>/dev/null || true

# --- Deps ---
sudo -n apt-get update -y
sudo -n apt-get install -y --no-install-recommends \
  ca-certificates curl jq git build-essential pkg-config libssl-dev ufw

# --- Fresh clone ---
sudo -n mkdir -p /opt
sudo -n rm -rf /opt/ippan
sudo -n git clone --depth 1 --branch "$BRANCH" "$REPO_URL" /opt/ippan || \
  sudo -n git clone --depth 1 "$REPO_URL" /opt/ippan
sudo -n chown -R "$(id -u):$(id -g)" /opt/ippan

cd /opt/ippan
# Try to checkout requested branch, then master/main if not already on it
git checkout "$BRANCH" 2>/dev/null || git checkout master 2>/dev/null || git checkout main 2>/dev/null || true
git rev-parse --short HEAD

# --- Run your existing setup script (authoritative) ---
# Check if setup script exists, try multiple paths/branches
SETUP_SCRIPT=""
if [ -f "./deploy/hetzner/scripts/setup-node.sh" ]; then
  SETUP_SCRIPT="./deploy/hetzner/scripts/setup-node.sh"
elif [ -f "deploy/hetzner/scripts/setup-node.sh" ]; then
  SETUP_SCRIPT="deploy/hetzner/scripts/setup-node.sh"
else
  echo "[remote] setup-node.sh not found in current branch, trying master/main..."
  git fetch origin master main 2>/dev/null || true
  git checkout master 2>/dev/null || git checkout main 2>/dev/null || true
  if [ -f "./deploy/hetzner/scripts/setup-node.sh" ]; then
    SETUP_SCRIPT="./deploy/hetzner/scripts/setup-node.sh"
  elif [ -f "deploy/hetzner/scripts/setup-node.sh" ]; then
    SETUP_SCRIPT="deploy/hetzner/scripts/setup-node.sh"
  else
    echo "[remote] ERROR: setup-node.sh not found in any branch. Available deploy scripts:"
    find deploy -name "*.sh" -type f 2>/dev/null | head -10 || true
    echo "[remote] Falling back to manual setup steps..."
    # Fallback: do basic setup inline (install deps, build, configure)
    sudo -n apt-get install -y clang cmake || true
    if ! command -v rustc &> /dev/null; then
      curl https://sh.rustup.rs -sSf | sh -s -- -y
      source $HOME/.cargo/env || true
    fi
    cargo build --release -p ippan-node || { echo "[remote] Build failed"; exit 1; }
    sudo -n mkdir -p /etc/ippan /var/lib/ippan /var/log/ippan
    sudo -n chown -R "$(id -u):$(id -g)" /etc/ippan /var/lib/ippan /var/log/ippan
    sudo -n cp target/release/ippan-node /usr/local/bin/ippan-node || { echo "[remote] Copy binary failed"; exit 1; }
    # Create minimal config
    sudo -n mkdir -p /etc/ippan/config
    cat > /tmp/node.toml << 'EOFCONFIG'
[node]
id = "ippan-devnet-${ROLE}"
[network]
id = "ippan-devnet"
[rpc]
host = "0.0.0.0"
port = 8080
[p2p]
host = "0.0.0.0"
port = 9000
EOFCONFIG
    if [ "$ROLE" != "node1" ] && [ -n "$BOOTSTRAP_IP" ]; then
      echo "bootstrap_nodes = [\"http://${BOOTSTRAP_IP}:9000\"]" >> /tmp/node.toml
    fi
    sudo -n mv /tmp/node.toml /etc/ippan/config/node.toml
    # Create systemd service (minimal)
    sudo -n tee /etc/systemd/system/ippan-node.service > /dev/null << 'EOFSERVICE'
[Unit]
Description=IPPAN Node
After=network.target

[Service]
Type=simple
User=ippan-devnet
WorkingDirectory=/var/lib/ippan
ExecStartPre=/bin/rm -f /var/lib/ippan/data/devnet/ippan-node.pid
ExecStart=/usr/local/bin/ippan-node --config /etc/ippan/config/node.toml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOFSERVICE
    # Don't exit - continue to firewall and service start steps below
    SETUP_COMPLETE=1
  fi
fi

if [ "${SETUP_COMPLETE:-0}" != "1" ]; then
  chmod +x "$SETUP_SCRIPT"
  if [ "$ROLE" = "node1" ]; then
    sudo -n "$SETUP_SCRIPT" node1
  else
    sudo -n "$SETUP_SCRIPT" "$ROLE" "$BOOTSTRAP_IP"
  fi
fi

# --- Firewall (do not lock out SSH) ---
sudo -n ufw allow 22/tcp || true

# Allow node-to-node ports between the 4 public IPs (both rpc+ p2p if you use them)
for IP in "$NODE1" "$NODE2" "$NODE3" "$NODE4"; do
  sudo -n ufw allow from "$IP" to any port 8080 proto tcp || true
  sudo -n ufw allow from "$IP" to any port 9000 proto tcp || true
  sudo -n ufw allow from "$IP" to any port 9000 proto udp || true
done

# node4: restrict RPC(8080) to laptop only (keep node-to-node rule already added above)
if [ "$ROLE" = "node4" ]; then
  sudo -n ufw delete allow 8080/tcp 2>/dev/null || true
  sudo -n ufw allow from "${LAPTOP_IP}/32" to any port 8080 proto tcp || true
fi

# --- Force systemd service to run as ippan-devnet (prevents Permission denied) ---
sudo -n mkdir -p /etc/systemd/system/ippan-node.service.d
cat <<'EOF' | sudo -n tee /etc/systemd/system/ippan-node.service.d/10-user.conf >/dev/null
[Service]
User=ippan-devnet
Group=ippan-devnet
EOF
sudo -n systemctl daemon-reload

# --- Ensure directories exist and are owned by ippan-devnet BEFORE restart (fix pid/log/data perms) ---
sudo -n mkdir -p /etc/ippan /etc/ippan/config /var/lib/ippan /var/log/ippan
sudo -n chown -R ippan-devnet:ippan-devnet /etc/ippan /var/lib/ippan /var/log/ippan
sudo -n chmod 750 /etc/ippan /etc/ippan/config /var/lib/ippan /var/log/ippan || true
sudo -n rm -f /var/lib/ippan/ippan-node.pid || true
sudo -n rm -f /var/lib/ippan/data/devnet/ippan-node.pid || true
sudo -n chown ippan-devnet:ippan-devnet /usr/local/bin/ippan-node 2>/dev/null || true
sudo -n chmod 755 /usr/local/bin/ippan-node 2>/dev/null || true

# --- Start service ---
sudo -n systemctl enable ippan-node
sudo -n systemctl restart ippan-node

# --- Quick local checks (bounded, no infinite loop) ---
for i in 1 2 3 4 5 6; do
  if curl -fsS http://127.0.0.1:8080/status >/dev/null 2>&1; then
    echo "[remote] status_ok"
    break
  fi
  sleep 2
done

echo "[remote] status:"
curl -fsS http://127.0.0.1:8080/status || true

echo "[remote] last journal lines:"
sudo -n journalctl -u ippan-node -n 80 --no-pager || true
'@

# Write remote script to a temp file locally (UTF8 without BOM for bash compatibility)
$RemotePath = Join-Path $Root "remote_redeploy.sh"
$Utf8NoBomEncoding = New-Object System.Text.UTF8Encoding $false
[System.IO.File]::WriteAllText($RemotePath, $RemoteScript, $Utf8NoBomEncoding)

Log "Starting clean redeploy. Evidence dir: $Root"
Log "Laptop public IP for node4 allowlist: $LaptopIP"
Log "Bootstrap public IP: $BootstrapIP"

foreach ($n in $Nodes) {
  $ip = $n.ip
  $name = $n.name
  $role = $n.role
  $ndir = Join-Path $Root $name
  New-Item -ItemType Directory -Force -Path $ndir | Out-Null

  Log "PRECHECK $name ($ip): ssh + sudo"
  (Ssh $ip '"bash -lc ""whoami; sudo -n true; echo SUDO_OK"" "') | Set-Content -Encoding UTF8 (Join-Path $ndir "precheck.txt")

  Log "RUN $name ($ip): wipe + redeploy"
  $envPrefix = "ROLE=$role BOOTSTRAP_IP=$BootstrapIP LAPTOP_IP=$LaptopIP NODE_PUBLIC_IP=$ip NODE1=$Node1 NODE2=$Node2 NODE3=$Node3 NODE4=$Node4 REPO_URL=$RepoUrl BRANCH=$Branch"
  $remoteTmpFile = "/tmp/redeploy_$(Get-Date -Format 'yyyyMMddHHmmss').sh"
  # Upload script via scp
  $scpCmd = "scp -o BatchMode=yes -o ConnectTimeout=8 {0} {1}@{2}:{3}" -f $RemotePath, $User, $ip, $remoteTmpFile
  try {
    $scpOut = & cmd.exe /c $scpCmd 2>&1
  } catch {
    Log "SCP upload failed for $name : $($_.Exception.Message)"
  }
  # Execute script with environment variables
  $execCmd = "{0} bash -x {1} 2>&1" -f $envPrefix, $remoteTmpFile
  $cleanupCmd = "rm -f {0}" -f $remoteTmpFile
  $fullCmd = "{0}; {1}" -f $execCmd, $cleanupCmd
  $sshCmd = "ssh -o BatchMode=yes -o ConnectTimeout=8 {0}@{1} ""{2}""" -f $User, $ip, $fullCmd
  $outFile = Join-Path $ndir "deploy_raw.txt"
  try {
    $process = Start-Process -FilePath "cmd.exe" -ArgumentList "/c", $sshCmd -RedirectStandardOutput $outFile -RedirectStandardError (Join-Path $ndir "deploy_err.txt") -NoNewWindow -Wait -PassThru
    $out = Get-Content $outFile -Raw -ErrorAction SilentlyContinue
    if (-not $out) { $out = "No output captured. Exit code: $($process.ExitCode)" }
  } catch {
    $out = "Error: $($_.Exception.Message)"
  }
  $out | Set-Content -Encoding UTF8 (Join-Path $ndir "deploy.txt")
}

Log "FINAL: Validate from laptop (node4 public RPC)"
try {
  $final = & cmd.exe /c ("curl -fsS http://{0}:8080/status" -f $Node4) 2>&1
  ($final -join "`r`n") | Tee-Object -FilePath (Join-Path $Root "node4_status_from_laptop.txt") | Out-Host
} catch {
  $errMsg = "Validation failed: $($_.Exception.Message)"
  $errMsg | Tee-Object -FilePath (Join-Path $Root "node4_status_from_laptop.txt") | Out-Host
  Log $errMsg
}

Log "DONE. If something fails, open tmp/devnet/redeploy_*/<node>/deploy.txt"
