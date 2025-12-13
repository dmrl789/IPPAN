param(
  [string]$User="ippan-devnet",
  [string]$Node1="188.245.97.41",
  [string]$Node2="135.181.145.174",
  [string]$Node3="5.223.51.238",
  [string]$Node4="178.156.219.107"
)

Set-StrictMode -Version Latest
$ErrorActionPreference="Stop"

$Hosts = @($Node1,$Node2,$Node3,$Node4)

function Ssh([string]$h,[string]$cmd){
  $sshTarget = "${User}@${h}"
  # Pass command as array element to ensure it's treated as single argument
  $result = & ssh.exe $sshTarget @($cmd) 2>&1
  return $result
}

$ts = Get-Date -Format "yyyyMMdd_HHmmss"
$Root = "tmp/devnet/node_recover_$ts"
New-Item -ItemType Directory -Force -Path $Root | Out-Null

function Log([string]$msg){
  $line = "[{0}] {1}" -f (Get-Date -Format "s"), $msg
  $line | Tee-Object -FilePath (Join-Path $Root "run.log") -Append | Out-Host
}

function RemoteConfigPath([string]$h){
  # Use a simpler approach: try common paths first, then systemd
  $bashCmd = @'
set -e
EXEC=$(sudo -n systemctl show ippan-node -p ExecStart --value 2>/dev/null || true)
CFG=""
if echo "$EXEC" | grep -q -- "--config"; then
  CFG=$(echo "$EXEC" | sed -E "s/.*--config[= ]([^ ]+).*/\1/" | tr -d '"')
fi
if [ -z "$CFG" ] || [ ! -f "$CFG" ]; then
  [ -f /etc/ippan/config/node.toml ] && CFG=/etc/ippan/config/node.toml
fi
if [ -z "$CFG" ]; then
  [ -f /etc/ippan/config/ippan-node.toml ] && CFG=/etc/ippan/config/ippan-node.toml
fi
if [ -z "$CFG" ]; then
  [ -f /etc/ippan/config/devnet.toml ] && CFG=/etc/ippan/config/devnet.toml
fi
echo "$CFG"
'@
  $bashCmd = $bashCmd.Replace("`r","").Replace('"','\"')
  $out = Ssh $h ("bash -lc " + '"' + $bashCmd + '"')
  $cfg = ($out | Select-Object -Last 1).Trim()
  if (-not $cfg) { return "" }
  return $cfg
}

function SnapshotNode([string]$h,[string]$tag){
  $d = Join-Path $Root ($h.Replace(".","_"))
  New-Item -ItemType Directory -Force -Path $d | Out-Null

  $verifyScript = @'
echo HOST:; hostname; echo UTC:; date -u; echo ---; echo ACTIVE:; sudo -n systemctl is-active ippan-node || true; echo ---; echo STATUS:; curl -fsS http://127.0.0.1:8080/status || true; echo; echo ---; echo PEERS:; curl -fsS http://127.0.0.1:8080/p2p/peers || true; echo
'@
  $verifyScript = $verifyScript.Replace("`r","").Replace("'","'\''")
  $bashCmd = "bash -lc '$verifyScript'"
  Ssh $h $bashCmd | Set-Content -Encoding UTF8 (Join-Path $d "$tag`_verify.txt")

  $listenersScript = @'
echo LISTENERS:; ss -lntp 2>/dev/null || true; echo ---; ss -lnup 2>/dev/null || true
'@
  $listenersScript = $listenersScript.Replace("`r","").Replace("'","'\''")
  $listenersCmd = "bash -lc '$listenersScript'"
  Ssh $h $listenersCmd | Set-Content -Encoding UTF8 (Join-Path $d "$tag`_listeners.txt")

  $journalScript = @'
sudo -n journalctl -u ippan-node -n 220 --no-pager || true
'@
  $journalScript = $journalScript.Replace("`r","").Replace("'","'\''")
  $journalCmd = "bash -lc '$journalScript'"
  Ssh $h $journalCmd | Set-Content -Encoding UTF8 (Join-Path $d "$tag`_journal_tail.txt")

  $unitScript = @'
echo UNIT:; sudo -n systemctl cat ippan-node || true; echo ---; echo CFG_DIR:; ls -la /etc/ippan/config 2>/dev/null || true
'@
  $unitScript = $unitScript.Replace("`r","").Replace("'","'\''")
  $unitCmd = "bash -lc '$unitScript'"
  Ssh $h $unitCmd | Set-Content -Encoding UTF8 (Join-Path $d "$tag`_unit_and_cfgdir.txt")
}

Log "STEP 1: PRE snapshot all nodes."
foreach($h in $Hosts){ SnapshotNode $h "pre" }

Log "STEP 2: Recover node1 (stop -> clear pid -> start) + ensure bootstrap_nodes empty on node1."
$cfg1 = RemoteConfigPath $Node1
$cfg1Display = if ($cfg1) { $cfg1 } else { "EMPTY" }
Log ("node1 config path detected: {0}" -f $cfg1Display)

# Stop + clear stale pidfiles, then start
$recoverScript = @'
set -e
sudo -n systemctl stop ippan-node || true
sudo -n rm -f /var/lib/ippan/ippan-node.pid /var/lib/ippan/node.pid /var/lib/ippan/*.pid 2>/dev/null || true
sudo -n systemctl start ippan-node
'@
$recoverScript = $recoverScript.Replace("`r","").Replace('"','\"')
Ssh $Node1 ("bash -lc " + '"' + $recoverScript + '"') |
  Set-Content -Encoding UTF8 (Join-Path $Root "node1_recover_stop_start.log")

# If we found config, force bootstrap_nodes="" (safe, minimal)
if ($cfg1) {
  $node1PatchScript = @'
set -euo pipefail
CFG="$1"
if [ -z "$CFG" ] || [ ! -f "$CFG" ]; then
  echo "ERROR: config not found: $CFG"
  exit 2
fi
TS="$(date -u +%Y%m%dT%H%M%SZ)"
sudo -n cp -a "$CFG" "$CFG.bak.$TS"
python3 - "$CFG" <<'PY'
import sys,re
path=sys.argv[1]
lines=open(path,'r',encoding='utf-8').read().splitlines(True)
out=[]
found=False
for ln in lines:
  if re.match(r'^\s*bootstrap_nodes\s*=\s*".*"\s*$', ln):
    out.append('bootstrap_nodes = ""\n')
    found=True
  else:
    out.append(ln)
if not found:
  out.append('\nbootstrap_nodes = ""\n')
open(path,'w',encoding='utf-8').write(''.join(out))
print("OK: bootstrap_nodes empty")
PY
echo CFG_NOW:
grep -n "bootstrap_nodes" "$CFG" || true
'@
  $payload = $node1PatchScript.Replace("`r","").Replace('"','\"')
  $escapedCfg = $cfg1.Replace('"','\"')
  $fullCmd = $payload + " " + $escapedCfg
  Ssh $Node1 ("bash -lc " + '"' + $fullCmd + '"') |
    Set-Content -Encoding UTF8 (Join-Path $Root "node1_patch_bootstrap_nodes.log")
} else {
  Log "WARNING: node1 config path is empty; cannot patch bootstrap_nodes. Evidence will show unit/config dir."
}

Log "STEP 3: Detect node1 REAL listening ports AFTER restart (choose bootstrap addr)."
Start-Sleep -Seconds 3
$listen1 = Ssh $Node1 "bash -lc 'ss -lntp 2>/dev/null || true; echo ---; ss -lnup 2>/dev/null || true'"
$listen1 | Set-Content -Encoding UTF8 (Join-Path $Root "node1_poststart_listeners.txt")

$node1Has9000 = $false
$node1Has8080 = $false
# Check for listening on ports - look for :9000 or :8080 in LISTEN state
if ($listen1 -match ":9000\s") { $node1Has9000 = $true }
if ($listen1 -match ":8080\s") { $node1Has8080 = $true }

$BootAddr = ""
if ($node1Has9000) { $BootAddr = "http://$Node1:9000" }
elseif ($node1Has8080) { $BootAddr = "http://$Node1:8080" }
else { $BootAddr = "http://$Node1:8080" } # fallback for safety

Log ("node1 listen: 9000={0} 8080={1} -> bootstrap addr={2}" -f $node1Has9000,$node1Has8080,$BootAddr)

Log "STEP 4: Force node2/node3/node4 bootstrap_nodes to $BootAddr (config-only)."

$patchBootstrap = @'
set -euo pipefail
CFG="$1"
BOOT="$2"
if [ -z "$CFG" ] || [ ! -f "$CFG" ]; then
  echo "ERROR: config not found: $CFG"
  exit 2
fi
TS="$(date -u +%Y%m%dT%H%M%SZ)"
sudo -n cp -a "$CFG" "$CFG.bak.$TS"
python3 - "$CFG" "$BOOT" <<'PY'
import sys,re
path=sys.argv[1]
boot=sys.argv[2]
lines=open(path,'r',encoding='utf-8').read().splitlines(True)
out=[]
found=False
for ln in lines:
  if re.match(r'^\s*bootstrap_nodes\s*=\s*".*"\s*$', ln):
    out.append(f'bootstrap_nodes = "{boot}"\n')
    found=True
  else:
    out.append(ln)
if not found:
  out.append(f'\nbootstrap_nodes = "{boot}"\n')
open(path,'w',encoding='utf-8').write(''.join(out))
print("OK")
PY
echo CFG_NOW:
grep -n "bootstrap_nodes" "$CFG" || true
'@

foreach($h in @($Node2,$Node3,$Node4)){
  $cfg = RemoteConfigPath $h
  Log "Patching bootstrap_nodes on $h cfg=$cfg"
  if (-not $cfg) {
    ("ERROR: Could not detect config on {0}" -f $h) | Tee-Object -FilePath (Join-Path $Root "bootstrap_patch_errors.log") -Append | Out-Host
    continue
  }
  $payload = $patchBootstrap.Replace("`r","").Replace('"','\"')
  Ssh $h ("bash -lc " + '"' + $payload + " " + $cfg + " " + $BootAddr + '"') |
    Set-Content -Encoding UTF8 (Join-Path $Root ("bootstrap_patch_{0}.log" -f $h.Replace(".","_")))
}

Log "STEP 5: UFW allow node-to-node on BOTH 8080 and 9000; node4 also allow laptop IP to 8080."
$myip=""
try { $myip=(curl.exe -fsS https://api.ipify.org).Trim() } catch { $myip="" }
$mycidr = ""
if ($myip) { $mycidr="$myip/32" }
$laptopIpDisplay = if ($mycidr) { $mycidr } else { "UNKNOWN" }
Log ("Laptop IP CIDR for node4: {0}" -f $laptopIpDisplay)

$ufwFix = @'
set -euo pipefail
NODE_SELF="$1"
ALLOW_LAPTOP="$2"

N1="188.245.97.41"
N2="135.181.145.174"
N3="5.223.51.238"
N4="178.156.219.107"

sudo -n ufw --force enable >/dev/null 2>&1 || true
sudo -n ufw allow 22/tcp >/dev/null 2>&1 || true

# allow intra-devnet + private net
for ip in "$N1" "$N2" "$N3" "$N4" "10.0.0.0/24"; do
  sudo -n ufw allow from "$ip" to any port 8080 proto tcp >/dev/null 2>&1 || true
  sudo -n ufw allow from "$ip" to any port 9000 proto tcp >/dev/null 2>&1 || true
  sudo -n ufw allow from "$ip" to any port 9000 proto udp >/dev/null 2>&1 || true
done

# node4: allow laptop IP to 8080 as well
if [ "$NODE_SELF" = "$N4" ] && [ -n "$ALLOW_LAPTOP" ]; then
  sudo -n ufw allow from "$ALLOW_LAPTOP" to any port 8080 proto tcp >/dev/null 2>&1 || true
fi

sudo -n ufw status verbose || true
'@

foreach($h in $Hosts){
  Log "Apply UFW fix on $h"
  $payload = $ufwFix.Replace("`r","").Replace('"','\"')
  Ssh $h ("bash -lc " + '"' + $payload + " " + $h + " " + $mycidr + '"') |
    Set-Content -Encoding UTF8 (Join-Path $Root ("ufw_{0}.txt" -f $h.Replace(".","_")))
}

Log "STEP 6: Restart order (leaf -> observer -> bootstrap): node2,node3,node4,node1."
$restartScript = "sudo -n systemctl restart ippan-node"
$restartScript = $restartScript.Replace("'","'\''")
$restartCmd = "bash -lc '$restartScript'"
Ssh $Node2 $restartCmd
Ssh $Node3 $restartCmd
Ssh $Node4 $restartCmd
Ssh $Node1 $restartCmd

Log "STEP 7: POST snapshot all nodes (no waiting loops)."
foreach($h in $Hosts){ SnapshotNode $h "post" }

Log "DONE. Evidence bundle: $Root"

