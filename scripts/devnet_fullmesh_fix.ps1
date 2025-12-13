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
  # Use PowerShell's native invocation with proper escaping
  $sshTarget = "${User}@${h}"
  $result = & ssh.exe $sshTarget $cmd 2>&1
  return $result
}

$ts = Get-Date -Format "yyyyMMdd_HHmmss"
$Root = "tmp/devnet/fullmesh_fix_$ts"
New-Item -ItemType Directory -Force -Path $Root | Out-Null

function Log([string]$msg){
  $line = "[{0}] {1}" -f (Get-Date -Format "s"), $msg
  $line | Tee-Object -FilePath (Join-Path $Root "run.log") -Append | Out-Host
}

function RemoteConfigPath([string]$h){
  $bashCmd = @'
set -e
EXEC=$(sudo -n systemctl show ippan-node -p ExecStart --value || true)
CFG=""
if echo "$EXEC" | grep -q -- "--config"; then
  CFG=$(echo "$EXEC" | sed -E "s/.*--config[= ]([^ ]+).*/\1/" | tr -d '"')
fi
if [ -z "$CFG" ] || [ ! -f "$CFG" ]; then
  [ -f /etc/ippan/config/node.toml ] && CFG=/etc/ippan/config/node.toml
fi
echo "$CFG"
'@
  $bashCmd = $bashCmd.Replace("`r","").Replace('"','\"')
  $cmd = "bash -lc " + '"' + $bashCmd + '"'
  $out = Ssh $h $cmd
  $cfg = ($out | Select-Object -Last 1).Trim()
  return $cfg
}

Log "STEP 1: Detect which ports node1 is listening on (8080 vs 9000)."
$listen = Ssh $Node1 "bash -lc 'ss -lntp 2>/dev/null || true; echo ---; ss -lnup 2>/dev/null || true'"
$listen | Set-Content -Encoding UTF8 (Join-Path $Root "node1_listeners.txt")

$node1Has9000 = $false
if ($listen -match "[:\\[]9000[\\]\\s]") { $node1Has9000 = $true }

Log ("DEBUG: Node1 value = '{0}'" -f $Node1)
if ($node1Has9000) { 
  $BootAddr = "http://${Node1}:9000" 
} else { 
  $BootAddr = "http://${Node1}:8080" 
}

Log ("node1Has9000={0} -> Using bootstrap addr: {1}" -f $node1Has9000, $BootAddr)

Log "STEP 2: Snapshot pre-fix status on all nodes (local-only via SSH)."
foreach($h in $Hosts){
  $d = Join-Path $Root ($h.Replace(".","_"))
  New-Item -ItemType Directory -Force -Path $d | Out-Null

  Ssh $h "bash -lc 'hostname; date -u; echo ---; sudo -n systemctl is-active ippan-node || true; curl -fsS http://127.0.0.1:8080/status || true; echo; curl -fsS http://127.0.0.1:8080/p2p/peers || true; echo'" |
    Set-Content -Encoding UTF8 (Join-Path $d "pre_verify.txt")

  Ssh $h "bash -lc 'sudo -n journalctl -u ippan-node -n 120 --no-pager || true'" |
    Set-Content -Encoding UTF8 (Join-Path $d "pre_journal_tail.txt")
}

Log "STEP 3: Ensure bootstrap_nodes are correct (node2/3/4 -> node1). node1 stays empty."

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
echo "Backup: $CFG.bak.$TS"

python3 - "$CFG" "$BOOT" <<'PY'
import sys,re
path=sys.argv[1]
boot=sys.argv[2]
with open(path,'r',encoding='utf-8') as f: lines=f.readlines()

out=[]
changed=False
found=False
for line in lines:
    if re.match(r'^\s*bootstrap_nodes\s*=\s*".*"\s*$', line):
        out.append(f'bootstrap_nodes = "{boot}"\n')
        changed=True
        found=True
    else:
        out.append(line)

if not found:
    # append at end (safe for TOML)
    out.append(f'\nbootstrap_nodes = "{boot}"\n')
    changed=True

with open(path,'w',encoding='utf-8') as f: f.writelines(out)
print("OK: bootstrap_nodes set")
PY

echo "CFG_NOW:"
grep -n "bootstrap_nodes" "$CFG" || true
'@

# node1: set bootstrap_nodes to empty string if present (keep bootstrap role clean)
$cfg1 = RemoteConfigPath $Node1
if ($cfg1) {
  Log "node1 config: $cfg1 (ensuring bootstrap_nodes is empty)"
  $node1PatchScript = @'
set -euo pipefail
CFG="$1"

if [ -z "$CFG" ] || [ ! -f "$CFG" ]; then
  echo "ERROR: config not found: $CFG"
  exit 2
fi

TS="$(date -u +%Y%m%dT%H%M%SZ)"
sudo -n cp -a "$CFG" "$CFG.bak.$TS"
echo "Backup: $CFG.bak.$TS"

python3 - "$CFG" <<'PY'
import sys,re
path=sys.argv[1]
with open(path,'r',encoding='utf-8') as f: lines=f.readlines()

out=[]
found=False
for line in lines:
    if re.match(r'^\s*bootstrap_nodes\s*=\s*".*"\s*$', line):
        out.append('bootstrap_nodes = ""\n')
        found=True
    else:
        out.append(line)

if not found:
    out.append('\nbootstrap_nodes = ""\n')

with open(path,'w',encoding='utf-8') as f: f.writelines(out)
print("OK: bootstrap_nodes cleared")
PY

echo "CFG_NOW:"
grep -n "bootstrap_nodes" "$CFG" || true
'@
  $payload = $node1PatchScript.Replace("`r","").Replace('"','\"')
  Ssh $Node1 ("bash -lc " + '"' + $payload + " " + $cfg1 + '"') |
    Set-Content -Encoding UTF8 (Join-Path $Root "node1_bootstrap_patch.log")
}

# node2/3/4: force to BootAddr
foreach($h in @($Node2,$Node3,$Node4)){
  $cfg = RemoteConfigPath $h
  Log "Patching bootstrap_nodes on $h config=$cfg -> $BootAddr"
  if (-not $cfg) {
    ("ERROR: Could not detect config on {0}" -f $h) | Tee-Object -FilePath (Join-Path $Root "bootstrap_patch_errors.log") -Append | Out-Host
    continue
  }
  $payload = $patchBootstrap.Replace("`r","").Replace('"','\"')
  Ssh $h ("bash -lc " + '"' + $payload + " " + $cfg + " " + $BootAddr + '"') |
    Set-Content -Encoding UTF8 (Join-Path $Root ("bootstrap_patch_{0}.log" -f $h.Replace(".","_")))
}

Log "STEP 4: Fix UFW rules so nodes can talk to each other on BOTH 8080 and 9000 (node4 also allows your laptop IP)."
# laptop IP for node4 RPC allowlist
$myip = ""
try { $myip = (curl.exe -fsS https://api.ipify.org).Trim() } catch { $myip = "" }
$mycidr = ""
if ($myip) { $mycidr = "$myip/32" }
$laptopIpDisplay = if ($mycidr) { $mycidr } else { "UNKNOWN" }
Log ("Laptop public IP (for node4 allowlist): {0}" -f $laptopIpDisplay)

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

# delete overly-broad "Anywhere" allows (best-effort, non-fatal)
NUMBERS="$(sudo -n ufw status numbered | sed -n 's/^\[\([0-9]\+\)\]\s\+\(8080\/tcp\|9000\/tcp\|9000\/udp\)\s\+ALLOW IN\s\+Anywhere.*/\1/p' | tac)"
for n in $NUMBERS; do
  echo y | sudo -n ufw delete "$n" >/dev/null 2>&1 || true
done

# allow intra-devnet on both ports
for ip in "$N1" "$N2" "$N3" "$N4" "10.0.0.0/24"; do
  sudo -n ufw allow from "$ip" to any port 8080 proto tcp >/dev/null 2>&1 || true
  sudo -n ufw allow from "$ip" to any port 9000 proto tcp >/dev/null 2>&1 || true
  sudo -n ufw allow from "$ip" to any port 9000 proto udp >/dev/null 2>&1 || true
done

# node4: allow laptop IP to 8080 as well (keep observer reachable)
if [ "$NODE_SELF" = "$N4" ] && [ -n "$ALLOW_LAPTOP" ]; then
  sudo -n ufw allow from "$ALLOW_LAPTOP" to any port 8080 proto tcp >/dev/null 2>&1 || true
fi

sudo -n ufw status verbose || true
'@

foreach($h in $Hosts){
  Log "Apply UFW fix on $h"
  $payload = $ufwFix.Replace("`r","").Replace('"','\"')
  $lip = $mycidr
  Ssh $h ("bash -lc " + '"' + $payload + " " + $h + " " + $lip + '"') |
    Set-Content -Encoding UTF8 (Join-Path $Root ("ufw_{0}.txt" -f $h.Replace(".","_")))
}

Log "STEP 5: Restart in deterministic order (node2,node3,node4,node1)."
Ssh $Node2 "bash -lc 'sudo -n systemctl restart ippan-node'"
Ssh $Node3 "bash -lc 'sudo -n systemctl restart ippan-node'"
Ssh $Node4 "bash -lc 'sudo -n systemctl restart ippan-node'"
Ssh $Node1 "bash -lc 'sudo -n systemctl restart ippan-node'"

Log "STEP 6: Post-fix verify (no loops; just capture)."
foreach($h in $Hosts){
  $d = Join-Path $Root ($h.Replace(".","_"))
  Ssh $h "bash -lc 'echo STATUS:; curl -fsS http://127.0.0.1:8080/status || true; echo; echo PEERS:; curl -fsS http://127.0.0.1:8080/p2p/peers || true; echo'" |
    Set-Content -Encoding UTF8 (Join-Path $d "post_verify.txt")

  Ssh $h "bash -lc 'sudo -n journalctl -u ippan-node -n 120 --no-pager || true'" |
    Set-Content -Encoding UTF8 (Join-Path $d "post_journal_tail.txt")
}

Log "DONE. Evidence bundle: $Root"

