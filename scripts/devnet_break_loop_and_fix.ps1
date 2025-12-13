param(
  [string]$User="ippan-devnet",
  [string[]]$Hosts=@("188.245.97.41","135.181.145.174","5.223.51.238","178.156.219.107")
)

Set-StrictMode -Version Latest
$ErrorActionPreference="Stop"

$ts = Get-Date -Format "yyyyMMdd_HHmmss"
$root = "tmp/devnet/breakloop_$ts"
New-Item -ItemType Directory -Force -Path $root | Out-Null

function Log([string]$msg){
  $line = "[{0}] {1}" -f (Get-Date -Format "s"), $msg
  $line | Tee-Object -FilePath (Join-Path $root "run.log") -Append | Out-Host
}

function Ssh([string]$h,[string]$cmd){
  # Use cmd.exe to avoid PS quoting edge-cases
  $oldEA = $ErrorActionPreference
  $ErrorActionPreference = "SilentlyContinue"
  $result = & cmd.exe /c "ssh $User@$h `"$cmd`"" 2>&1
  $ErrorActionPreference = $oldEA
  return $result
}

Log "STEP 1: Stop local PowerShell jobs / background loops (no waiting)."
$killNames = @("ippan_devnet_soak","monitor_then_train","ippan*","devnet*","auto*")
Get-Job -ErrorAction SilentlyContinue | ForEach-Object {
  try {
    if ($killNames | Where-Object { $_ -and ($_.ToLower()) -and ($_.ToLower() -like "*") }) { }
    Stop-Job -Id $_.Id -Force -ErrorAction SilentlyContinue | Out-Null
    Remove-Job -Id $_.Id -Force -ErrorAction SilentlyContinue | Out-Null
  } catch {}
}

# Kill stray powershell.exe that are running known devnet scripts (safe filter by CommandLine)
try {
  $procs = Get-CimInstance Win32_Process -Filter "Name='powershell.exe'"
  foreach($p in $procs){
    $cl = if ($p.CommandLine) { $p.CommandLine } else { "" }
    if ($cl -match "devnet_soak\.ps1|monitor_then_train\.ps1|auto_diverse_dgbdt\.ps1|run_full_dgbdt_pipeline\.ps1"){
      Log "Killing loop process PID=$($p.ProcessId) cmd=$cl"
      Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue
    }
  }
} catch {}

Log "STEP 2: TRIAGE SNAPSHOT (service + local status/peers + listeners + pidfiles)."
foreach($h in $Hosts){
  $dir = Join-Path $root ($h.Replace(".","_"))
  New-Item -ItemType Directory -Force -Path $dir | Out-Null

  try {
    Ssh $h "hostname; date -u; echo ---; sudo -n systemctl is-active ippan-node; sudo -n systemctl status ippan-node --no-pager -l | tail -n 120" |
      Set-Content -Encoding UTF8 (Join-Path $dir "systemd.txt")
  } catch {
    Log "Warning: Failed to get systemd status from $h : $_"
  }

  try {
    Ssh $h 'curl -fsS http://127.0.0.1:8080/status || true' |
      Set-Content -Encoding UTF8 (Join-Path $dir "status_8080.json")
  } catch {}

  try {
    Ssh $h 'curl -fsS http://127.0.0.1:8080/p2p/peers || true' |
      Set-Content -Encoding UTF8 (Join-Path $dir "p2p_peers_8080.json")
  } catch {}

  try {
    Ssh $h 'curl -fsS http://127.0.0.1:9000/p2p/peers || true' |
      Set-Content -Encoding UTF8 (Join-Path $dir "p2p_peers_9000.txt")
  } catch {}

  try {
    Ssh $h 'ss -lntp || true; echo ---; ss -lnup || true' |
      Set-Content -Encoding UTF8 (Join-Path $dir "listeners.txt")
  } catch {}

  try {
    Ssh $h 'ls -la /var/lib/ippan 2>/dev/null || true; ls -la /var/lib/ippan/*.pid 2>/dev/null || true' |
      Set-Content -Encoding UTF8 (Join-Path $dir "state_dir.txt")
  } catch {}

  try {
    Ssh $h "sudo -n journalctl -u ippan-node -n 250 --no-pager" |
      Set-Content -Encoding UTF8 (Join-Path $dir "journal_tail.txt")
  } catch {}
}

Log "STEP 3: FIX advertised P2P port mismatch safely (only if 8080 peers works and 9000 peers does NOT)."
$remoteFix = @'
set -euo pipefail

echo "== HOST: $(hostname) =="
sudo -n systemctl is-active ippan-node >/dev/null

EXEC="$(sudo -n systemctl show ippan-node -p ExecStart --value || true)"
CFG=""
if echo "$EXEC" | grep -q -- "--config"; then
  CFG="$(echo "$EXEC" | sed -E 's/.*--config[= ]([^ ]+).*/\1/' | tr -d '"')"
fi
if [ -z "$CFG" ] || [ ! -f "$CFG" ]; then
  if [ -f /etc/ippan/config/node.toml ]; then CFG="/etc/ippan/config/node.toml"; fi
fi
if [ -z "$CFG" ] || [ ! -f "$CFG" ]; then
  echo "ERROR: config not found. ExecStart=$EXEC"
  exit 2
fi
echo "Config: $CFG"

OK8080=0
OK9000=0
curl -fsS http://127.0.0.1:8080/p2p/peers >/dev/null 2>&1 && OK8080=1 || true
curl -fsS http://127.0.0.1:9000/p2p/peers >/dev/null 2>&1 && OK9000=1 || true
echo "p2p/peers: 8080=$OK8080 9000=$OK9000"

if [ "$OK8080" -eq 1 ] && [ "$OK9000" -eq 0 ]; then
  TS="$(date -u +%Y%m%dT%H%M%SZ)"
  sudo -n cp -a "$CFG" "$CFG.bak.$TS"
  echo "Backup: $CFG.bak.$TS"

  python3 - <<'PY'
import re,sys
path=sys.argv[1]
with open(path,'r',encoding='utf-8') as f: lines=f.readlines()

out=[]
in_p2p=False
patched=False
saw_p2p=False

for line in lines:
    m=re.match(r'^\s*\[(.+?)\]\s*$', line)
    if m:
        in_p2p=(m.group(1).strip()=="p2p")
        if in_p2p: saw_p2p=True
    if in_p2p and re.match(r'^\s*port\s*=\s*\d+\s*$', line):
        out.append('port = 8080\n')
        patched=True
    else:
        out.append(line)

if saw_p2p and not patched:
    out2=[]
    inserted=False
    for line in out:
        out2.append(line)
        if (not inserted) and re.match(r'^\s*\[p2p\]\s*$', line):
            out2.append('port = 8080\n')
            inserted=True
    out=out2

if (not saw_p2p):
    out.append('\n[p2p]\nport = 8080\n')

with open(path,'w',encoding='utf-8') as f: f.writelines(out)
print("OK: set [p2p].port=8080")
PY
"$CFG"

  sudo -n systemctl restart ippan-node
  sleep 2
  curl -fsS http://127.0.0.1:8080/p2p/peers >/dev/null
  echo "PATCH_OK"
else
  echo "SKIP_PATCH"
fi
'@

foreach($h in $Hosts){
  Log "Apply advertised-port fix on $h"
  $payload = $remoteFix.Replace("`r","").Replace('"','\"')
  Ssh $h ("bash -lc " + '"' + $payload + '"') | Tee-Object -FilePath (Join-Path $root "fix_$($h.Replace('.','_')).log") | Out-Host
}

Log "STEP 4: CLEAN REJOIN restart order (leaf nodes first, bootstrap last)."
$node1="188.245.97.41"
$node2="135.181.145.174"
$node3="5.223.51.238"
$node4="178.156.219.107"

Ssh $node2 "sudo -n systemctl restart ippan-node"
Ssh $node3 "sudo -n systemctl restart ippan-node"
Ssh $node4 "sudo -n systemctl restart ippan-node"
Ssh $node1 "sudo -n systemctl restart ippan-node"

Log "STEP 5: VERIFY (NO LOOPS) - local status+peers on each node via SSH."
foreach($h in $Hosts){
  Log "Verify on $h"
  $out = Ssh $h 'echo STATUS:; curl -fsS http://127.0.0.1:8080/status || true; echo; echo PEERS:; curl -fsS http://127.0.0.1:8080/p2p/peers || true; echo'
  $dir = Join-Path $root ($h.Replace(".","_"))
  $out | Set-Content -Encoding UTF8 (Join-Path $dir "verify.txt")
  $out | Out-Host
}

Log "DONE. Evidence bundle: $root"

