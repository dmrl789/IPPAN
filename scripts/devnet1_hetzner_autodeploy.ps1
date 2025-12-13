$ErrorActionPreference = "Stop"

$NODE1_PUB  = "188.245.97.41"
$NODE1_PRIV = "10.0.0.2"
$NODE2_PUB  = "135.181.145.174"
$NODE2_PRIV = "10.0.0.3"
$NODE3_PUB  = "5.223.51.238"
$NODE4_PUB  = "178.156.219.107"

$RepoUrl = "https://github.com/dmrl789/IPPAN"
$Branch  = "master"

# IMPORTANT: node2 uses node1 private; node3+node4 use node1 public
$Nodes = @(
  @{ Name="node1"; Host=$NODE1_PUB; Bootstrap=""           ; Role="bootstrap" },
  @{ Name="node2"; Host=$NODE2_PUB; Bootstrap=$NODE1_PRIV  ; Role="validator" },
  @{ Name="node3"; Host=$NODE3_PUB; Bootstrap=$NODE1_PUB   ; Role="validator" },
  @{ Name="node4"; Host=$NODE4_PUB; Bootstrap=$NODE1_PUB   ; Role="observer"  }
)

function Sh($hostname, $cmd) {
  # Use accept-new so we never get interactive host key prompts
  # Use BatchMode to fail fast if keys aren't configured (no password prompts)
  $sshArgs = @(
    "-o", "StrictHostKeyChecking=accept-new",
    "-o", "BatchMode=yes",
    "-o", "UserKnownHostsFile=$env:USERPROFILE\.ssh\known_hosts",
    "ippan@$hostname",
    $cmd
  )
  & ssh @sshArgs
  if ($LASTEXITCODE -ne 0) { throw "SSH command failed on $hostname (exit=$LASTEXITCODE): $cmd" }
}

Write-Host "== Preflight: SSH + sudo (NOPASSWD) ==" -ForegroundColor Cyan
foreach ($n in $Nodes) {
  $h = $n.Host
  Write-Host "-- $($n.Name) @ $h"
  Sh $h "whoami && hostname && uptime"
  # sudo -n must succeed for FULL automation
  Sh $h "sudo -n true && echo SUDO_OK"
}

Write-Host "== Sync repo on all nodes to /opt/ippan ==" -ForegroundColor Cyan
foreach ($n in $Nodes) {
  $h = $n.Host
  Write-Host "-- $($n.Name) @ $h"
  Sh $h "sudo -n mkdir -p /opt/ippan && sudo -n chown -R ippan:ippan /opt/ippan"
  Sh $h "cd /opt/ippan && (test -d .git && git fetch origin && git reset --hard origin/$Branch || git clone $RepoUrl .) && git checkout $Branch && git pull --ff-only"
  Sh $h "cd /opt/ippan && chmod +x deploy/hetzner/scripts/setup-node.sh || true"
}

Write-Host "== Run setup-node.sh on each node ==" -ForegroundColor Cyan
foreach ($n in $Nodes) {
  $h = $n.Host
  $name = $n.Name
  $bootstrap = $n.Bootstrap
  Write-Host "-- $name @ $h (bootstrap=$bootstrap)"
  if ([string]::IsNullOrWhiteSpace($bootstrap)) {
    Sh $h "cd /opt/ippan && bash -lc './deploy/hetzner/scripts/setup-node.sh $name'"
  } else {
    Sh $h "cd /opt/ippan && bash -lc './deploy/hetzner/scripts/setup-node.sh $name $bootstrap'"
  }
}

Write-Host "== Start services + open firewall ports (if ufw present) ==" -ForegroundColor Cyan
foreach ($n in $Nodes) {
  $h = $n.Host
  Write-Host "-- $($n.Name) @ $h"
  Sh $h "sudo -n systemctl daemon-reload"
  Sh $h "sudo -n systemctl enable ippan-node"
  Sh $h "sudo -n systemctl restart ippan-node"
  Sh $h "sleep 2; sudo -n systemctl --no-pager --full status ippan-node | head -n 30"

  # Firewall: allow SSH + RPC + P2P. Do NOT force-enable ufw (to avoid lockouts).
  Sh $h "if command -v ufw >/dev/null 2>&1; then sudo -n ufw allow 22/tcp || true; sudo -n ufw allow 8080/tcp || true; sudo -n ufw allow 9000/tcp || true; sudo -n ufw allow 9000/udp || true; sudo -n ufw status verbose || true; else echo 'ufw not installed; skipping'; fi"
}

Write-Host "== Validate: /status on node4 (RPC/observer) ==" -ForegroundColor Cyan
$node4 = $Nodes | Where-Object { $_.Name -eq "node4" }
$node4Host = $node4.Host

try {
  $status = Invoke-RestMethod -Uri "http://$node4Host:8080/status" -TimeoutSec 10
  Write-Host "node4 /status OK"
  $status | ConvertTo-Json -Depth 12
} catch {
  Write-Host "node4 /status FAILED. Showing node4 logs tail..." -ForegroundColor Yellow
  Sh $node4Host "sudo -n journalctl -u ippan-node -n 120 --no-pager || sudo -n tail -n 120 /var/log/ippan/node.log"
  throw
}

Write-Host "== Done. If validators/peers are still joining, tail logs ==" -ForegroundColor Green
Write-Host "Tip: ssh ippan@$node4Host 'sudo -n journalctl -u ippan-node -n 80 --no-pager'"

