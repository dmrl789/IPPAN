param(
  [string]$User = "ippan-devnet",
  [string[]]$Hosts = @("188.245.97.41","135.181.145.174","5.223.51.238","178.156.219.107")
)

$ErrorActionPreference = "Continue"
$stamp = Get-Date -Format "yyyyMMdd_HHmmss"
$outDir = Join-Path "tmp/devnet" "collect_$stamp"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

function RunSsh([string]$hostname, [string]$cmd, [string]$outFile) {
  Write-Host "[$hostname] Collecting: $outFile" -ForegroundColor Gray
  try {
    $result = & ssh -o StrictHostKeyChecking=accept-new "$User@$hostname" $cmd 2>&1
    $result | Out-File -Encoding utf8 $outFile
  } catch {
    "Error: $_" | Out-File -Encoding utf8 $outFile
  }
}

foreach($h in $Hosts){
  $safe = $h.Replace(".","_")
  $dir = Join-Path $outDir $safe
  New-Item -ItemType Directory -Force -Path $dir | Out-Null

  Write-Host "`n=== Collecting from $h ===" -ForegroundColor Cyan

  # Meta
  RunSsh $h "date -u && hostname && uname -a" (Join-Path $dir "01_meta.txt")
  
  # IP
  RunSsh $h "ip -br a" (Join-Path $dir "02_ip.txt")
  
  # Listeners
  RunSsh $h "sudo ss -lntup | grep -E ':(8080|9000)' || echo 'none'" (Join-Path $dir "03_listeners.txt")
  
  # UFW
  RunSsh $h "sudo ufw status verbose 2>/dev/null || echo 'ufw not installed'" (Join-Path $dir "04_ufw.txt")
  
  # Service status
  RunSsh $h "sudo systemctl is-active ippan-node && sudo systemctl status ippan-node --no-pager -n 30" (Join-Path $dir "05_service.txt")
  
  # Systemd unit
  RunSsh $h "sudo systemctl cat ippan-node" (Join-Path $dir "06_systemd_unit.txt")
  
  # Status endpoint
  RunSsh $h "curl -fsS http://127.0.0.1:8080/status 2>&1" (Join-Path $dir "07_status.json")
  
  # Config grep
  RunSsh $h "sudo grep -rIn --include='*.toml' --include='*.yaml' --include='*.json' 'bootstrap\|bootnode\|seed\|peer\|multiaddr\|listen\|p2p' /etc/ippan /opt/ippan /var/lib/ippan 2>/dev/null | head -n 200" (Join-Path $dir "08_config_grep.txt")
  
  # Config files
  RunSsh $h "sudo cat /etc/ippan/config/node.toml 2>/dev/null || echo 'not found'" (Join-Path $dir "09_node_toml.txt")
  
  # Logs (peer keywords)
  RunSsh $h "sudo journalctl -u ippan-node -n 500 --no-pager | grep -iE 'peer|connect|dial|listen|bootstrap|discov|swarm|handshake|identify|gossip|multiaddr|p2p' || echo 'no matches'" (Join-Path $dir "10_logs_peer_keywords.txt")
  
  # Raw logs
  RunSsh $h "sudo journalctl -u ippan-node -n 200 --no-pager" (Join-Path $dir "11_logs_raw.txt")
  
  # P2P peers endpoint
  RunSsh $h "curl -fsS http://127.0.0.1:8080/p2p/peers 2>&1" (Join-Path $dir "12_p2p_peers.json")
}

Write-Host "`n=== DONE ===" -ForegroundColor Green
Write-Host "Output directory: $outDir" -ForegroundColor Yellow
Write-Host "`nKey files to review:" -ForegroundColor Cyan
Write-Host "  - node1: 188_245_97_41/09_node_toml.txt, 07_status.json, 10_logs_peer_keywords.txt" -ForegroundColor Gray
Write-Host "  - node4: 178_156_219_107/09_node_toml.txt, 07_status.json, 10_logs_peer_keywords.txt" -ForegroundColor Gray

