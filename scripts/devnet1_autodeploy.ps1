$ErrorActionPreference="Stop"

$NODE1_PUB="188.245.97.41"; $NODE1_PRIV="10.0.0.2"
$NODE2_PUB="135.181.145.174"
$NODE3_PUB="5.223.51.238"
$NODE4_PUB="178.156.219.107"

$RepoUrl="https://github.com/dmrl789/IPPAN"
$Branch="master"

$Nodes=@(
  @{Name="node1"; Host=$NODE1_PUB; Bootstrap=""          },
  @{Name="node2"; Host=$NODE2_PUB; Bootstrap=$NODE1_PRIV },
  @{Name="node3"; Host=$NODE3_PUB; Bootstrap=$NODE1_PUB  },
  @{Name="node4"; Host=$NODE4_PUB; Bootstrap=$NODE1_PUB  }
)

function Sh($hostname,$cmd){
  & ssh -o StrictHostKeyChecking=accept-new "ippan-devnet@$hostname" $cmd
  if($LASTEXITCODE -ne 0){ throw "FAILED on $hostname : $cmd" }
}

Write-Host "== Preflight: ippan-devnet ssh + NOPASSWD sudo ==" -ForegroundColor Cyan
foreach($n in $Nodes){
  Sh $n.Host "whoami && sudo -n true && echo SUDO_OK && hostname"
}

Write-Host "== Sync repo to /opt/ippan on all nodes ==" -ForegroundColor Cyan
foreach($n in $Nodes){
  Sh $n.Host "sudo -n mkdir -p /opt/ippan && sudo -n chown -R ippan-devnet:ippan-devnet /opt/ippan"
  Sh $n.Host "cd /opt/ippan && (test -d .git && git fetch origin && git reset --hard origin/$Branch || git clone $RepoUrl .) && git checkout $Branch && git pull --ff-only"
  Sh $n.Host "cd /opt/ippan && chmod +x deploy/hetzner/scripts/setup-node.sh || true"
}

Write-Host "== Run setup-node.sh ==" -ForegroundColor Cyan
foreach($n in $Nodes){
  if([string]::IsNullOrWhiteSpace($n.Bootstrap)){
    Sh $n.Host "cd /opt/ippan && bash -lc './deploy/hetzner/scripts/setup-node.sh $($n.Name)'"
  } else {
    Sh $n.Host "cd /opt/ippan && bash -lc './deploy/hetzner/scripts/setup-node.sh $($n.Name) $($n.Bootstrap)'"
  }
}

Write-Host "== Start services ==" -ForegroundColor Cyan
foreach($n in $Nodes){
  Sh $n.Host "sudo -n systemctl daemon-reload"
  Sh $n.Host "sudo -n systemctl enable ippan-node"
  Sh $n.Host "sudo -n systemctl restart ippan-node"
  Sh $n.Host "sleep 2; sudo -n systemctl --no-pager --full status ippan-node | head -n 25"
}

Write-Host "== Open ports (do NOT enable ufw automatically) ==" -ForegroundColor Cyan
foreach($n in $Nodes){
  Sh $n.Host "if command -v ufw >/dev/null 2>&1; then sudo -n ufw allow 22/tcp || true; sudo -n ufw allow 8080/tcp || true; sudo -n ufw allow 9000/tcp || true; sudo -n ufw allow 9000/udp || true; sudo -n ufw status verbose || true; else echo 'ufw not installed'; fi"
}

Write-Host "== Validate node4 /status ==" -ForegroundColor Cyan
try{
  $s = Invoke-RestMethod -Uri "http://$NODE4_PUB:8080/status" -TimeoutSec 10
  $s | ConvertTo-Json -Depth 12
}catch{
  Write-Host "node4 status failed; showing node4 logs" -ForegroundColor Yellow
  Sh $NODE4_PUB "sudo -n journalctl -u ippan-node -n 120 --no-pager || true"
  throw
}

Write-Host "DEVNET-1 DEPLOY COMPLETE" -ForegroundColor Green

