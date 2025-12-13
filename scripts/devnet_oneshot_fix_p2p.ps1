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
$BootstrapRpc = "http://$Node1:8080"   # MUST NEVER BE EMPTY
if ([string]::IsNullOrWhiteSpace($BootstrapRpc) -or $BootstrapRpc -eq "http://:8080" -or $BootstrapRpc -match '^http://\s*:') { 
  throw "BOOTSTRAP_RPC computed empty or invalid: '$BootstrapRpc'. STOP." 
}

$ts = Get-Date -Format "yyyyMMdd_HHmmss"
$Root = "tmp/devnet/oneshot_fix_$ts"
New-Item -ItemType Directory -Force -Path $Root | Out-Null

function Log([string]$m){
  $line = "[{0}] {1}" -f (Get-Date -Format "s"), $m
  $line | Tee-Object -FilePath (Join-Path $Root "run.log") -Append | Out-Host
}

function Ssh([string]$h,[string]$cmd){
  # Never throw on SSH; capture output and continue
  $escapedCmd = $cmd.Replace('"', '""')
  $out = & cmd.exe /c ("ssh {0}@{1} ""{2}""" -f $User, $h, $escapedCmd) 2>&1
  return $out
}

function NodeDir([string]$h){
  $d = Join-Path $Root ($h.Replace(".","_"))
  New-Item -ItemType Directory -Force -Path $d | Out-Null
  return $d
}

# Remote: robust config finder (NO systemd parsing if it fails; fallback to grep in /etc/ippan/config)
# Outputs an absolute path or empty string.
function GetRemoteConfigPath([string]$h){
  $pyScript = @'
import os,subprocess,shlex,glob,sys,re

def sh(cmd):
  try:
    return subprocess.check_output(cmd, shell=True, text=True).strip()
  except Exception:
    return ""

# 1) Try systemd ExecStart --config
exec1 = sh("sudo -n systemctl show ippan-node -p ExecStart --value")
exec2 = sh("sudo -n systemctl cat ippan-node | sed -n \"s/^ExecStart=//p\" | head -n1")

def find_cfg(s):
  if not s: return None
  try:
    toks = shlex.split(s)
  except Exception:
    toks = s.split()
  for i,t in enumerate(toks):
    if t.startswith("--config="):
      return t.split("=",1)[1]
    if t == "--config" and i+1 < len(toks):
      return toks[i+1]
  return None

for s in (exec1, exec2):
  p = find_cfg(s)
  if p and os.path.isfile(p):
    print(p); sys.exit(0)

# 2) Common locations
cands = [
  "/etc/ippan/config/node.toml",
  "/etc/ippan/config/ippan-node.toml"
]
for p in cands:
  if os.path.isfile(p):
    print(p); sys.exit(0)

# 3) Fallback: find a TOML that contains [p2p]
gl = sorted(glob.glob("/etc/ippan/config/*.toml"))
for p in gl:
  try:
    txt=open(p,'r',encoding='utf-8',errors='ignore').read()
  except Exception:
    continue
  if re.search(r'^\[p2p\]\s*$', txt, re.M):
    print(p); sys.exit(0)

print(""); sys.exit(1)
'@
  $bytes = [System.Text.Encoding]::UTF8.GetBytes($pyScript)
  $b64 = [Convert]::ToBase64String($bytes)
  $cmd = "bash -lc 'echo $b64 | base64 -d | python3'"
  $out = Ssh $h $cmd
  $line = ($out | Select-Object -Last 1).Trim()
  return $line
}

# Determine actual p2p port by checking listeners (tcp+udp)
function GetActualP2PPort([string]$h){
  $cmd = 'bash -lc ''set -e; (ss -lntup || true) | sed -n "1,200p"'''
  $out = Ssh $h $cmd
  $txt = ($out -join "`n")
  # If 9000 is listening on tcp or udp, prefer 9000
  if ($txt -match '[:\.]9000\s') { return 9000 }
  return 8080
}

# Patch TOML safely: requires [p2p] section.
# Updates/inserts inside [p2p]: port, advertised_address, bootstrap_nodes
function PatchRemoteToml([string]$h,[string]$cfg,[int]$p2pPort,[string]$adv,[string]$bootstrap){
  if ([string]::IsNullOrWhiteSpace($cfg)) { throw "cfg empty for $h" }
  if ($adv -match '^http://\s*$') { throw "adv empty for $h" }
  if ($adv -notmatch ':\d+$') { throw "adv missing port for $h => $adv" }
  if ($bootstrap -ne "" -and $bootstrap -notmatch '^http://\d+\.\d+\.\d+\.\d+:\d+$') { throw "bootstrap malformed for $h => $bootstrap" }

  $py = @"
import sys,re,datetime

path=sys.argv[1]
p2p_port=int(sys.argv[2])
adv=sys.argv[3]
bootstrap=sys.argv[4]  # may be empty

lines=open(path,'r',encoding='utf-8',errors='ignore').read().splitlines(True)

# locate [p2p] section
sec_start=None
for i,ln in enumerate(lines):
  if re.match(r'^\\[p2p\\]\\s*$', ln.strip()):
    sec_start=i
    break
if sec_start is None:
  print('NO_P2P_SECTION')
  sys.exit(3)

# section end is next [section]
sec_end=len(lines)
for j in range(sec_start+1,len(lines)):
  if re.match(r'^\\[[^\\]]+\\]\\s*$', lines[j].strip()):
    sec_end=j
    break

p2p_block=lines[sec_start:sec_end]

def set_or_insert(block,key,val,is_str):
  pat=re.compile(r'^\\s*'+re.escape(key)+r'\\s*=')
  for k in range(len(block)):
    if pat.match(block[k]):
      block[k]=f'{key} = \"{val}\"\\n' if is_str else f'{key} = {val}\\n'
      return True
  # insert right after [p2p] header
  ins=1
  block.insert(ins, (f'{key} = \"{val}\"\\n' if is_str else f'{key} = {val}\\n'))
  return False

# hard rules: never write empty http://
if adv.strip()=='http://':
  print('ADV_EMPTY'); sys.exit(4)
if bootstrap.strip()=='http://':
  print('BOOTSTRAP_EMPTY'); sys.exit(4)

set_or_insert(p2p_block,'port',p2p_port,False)
set_or_insert(p2p_block,'advertised_address',adv,True)

# bootstrap_nodes is optional; if empty => remove existing key in [p2p]
pat_bn=re.compile(r'^\\s*bootstrap_nodes\\s*=')
p2p_block=[ln for ln in p2p_block if not pat_bn.match(ln)]
if bootstrap.strip():
  # add it (string)
  p2p_block.insert(2, f'bootstrap_nodes = \"{bootstrap}\"\\n')

# write back
out=lines[:sec_start]+p2p_block+lines[sec_end:]

bak=path+'.bak.'+datetime.datetime.utcnow().strftime('%Y%m%dT%H%M%SZ')
open(bak,'w',encoding='utf-8').write(''.join(lines))
open(path,'w',encoding='utf-8').write(''.join(out))
print('OK')
"@

  # execute patch on remote (copy via heredoc)
  $bytes = [System.Text.Encoding]::UTF8.GetBytes($py)
  $b64 = [Convert]::ToBase64String($bytes)
  $cmd = "bash -lc 'set -e; CFG=`"$cfg`"; TS=`$(date -u +%Y%m%dT%H%M%SZ); sudo -n cp -a `"`$CFG`" `"`$CFG.pre_oneshot.`"`$TS; echo $b64 | base64 -d | python3 - `"`$CFG`" `"$p2pPort`" `"$adv`" `"$bootstrap`"'"
  return (Ssh $h $cmd)
}

function Snapshot([string]$h,[string]$tag){
  $d = NodeDir $h
  $cfg = GetRemoteConfigPath $h
  $p2p = GetActualP2PPort $h
  $snap = @()
  $snap += "CFG_PATH=$cfg"
  $snap += "P2P_ACTUAL_PORT=$p2p"
  $snap += "STATUS:"
  $snap += (Ssh $h "bash -lc 'curl -fsS http://127.0.0.1:8080/status || true'")
  $snap += "PEERS:"
  $snap += (Ssh $h "bash -lc 'curl -fsS http://127.0.0.1:8080/p2p/peers || true'")
  $snap += "LISTENERS:"
  $snap += (Ssh $h "bash -lc 'ss -lntup 2>/dev/null || true'")
  ($snap -join "`n") | Set-Content -Encoding UTF8 (Join-Path $d "$tag`_snap.txt")

  if ($cfg) {
    (Ssh $h ("bash -lc 'sed -n ""1,160p"" ""{0}""'" -f $cfg)) |
      Set-Content -Encoding UTF8 (Join-Path $d "$tag`_cfg_head.txt")
  }
}

Log "STEP 1: PRE snapshots"
foreach($h in $Hosts){ Snapshot $h "pre" }

Log "STEP 2: Patch [p2p] on each node with correct keys (port/advertised_address/bootstrap_nodes)."
$advMap = @{
  $Node1 = "http://$Node1" + ":{PORT}"
  $Node2 = "http://$Node2" + ":{PORT}"
  $Node3 = "http://$Node3" + ":{PORT}"
  $Node4 = "http://$Node4" + ":{PORT}"
}

foreach($h in $Hosts){
  $d = NodeDir $h
  $cfg = GetRemoteConfigPath $h
  if ([string]::IsNullOrWhiteSpace($cfg)){
    Log "ERROR: config path not found on $h. STOP patch for this host."
    "CONFIG_NOT_FOUND" | Set-Content -Encoding UTF8 (Join-Path $d "patch_result.txt")
    continue
  }

  $p2pPort = GetActualP2PPort $h
  $adv = $advMap[$h].Replace("{PORT}", [string]$p2pPort)

  # bootstrap: node1 empty, others use BootstrapRpc
  $bootstrap = ""
  if ($h -ne $Node1) { $bootstrap = $BootstrapRpc }

  Log "Patch $h cfg=$cfg p2pPort=$p2pPort adv=$adv bootstrap=$bootstrap"
  $out = $null
  try {
    $out = PatchRemoteToml $h $cfg $p2pPort $adv $bootstrap
    ($out -join "`n") | Set-Content -Encoding UTF8 (Join-Path $d "patch_result.txt")
  } catch {
    ("PATCH_THROW: " + $_.Exception.Message) | Set-Content -Encoding UTF8 (Join-Path $d "patch_result.txt")
  }
}

Log "STEP 3: Restart order (node2,node3,node4,node1). No waiting loops."
Ssh $Node2 "bash -lc 'sudo -n systemctl restart ippan-node'" | Out-Null
Ssh $Node3 "bash -lc 'sudo -n systemctl restart ippan-node'" | Out-Null
Ssh $Node4 "bash -lc 'sudo -n systemctl restart ippan-node'" | Out-Null
Ssh $Node1 "bash -lc 'sudo -n systemctl restart ippan-node'" | Out-Null

Start-Sleep -Seconds 4

Log "STEP 4: POST snapshots"
foreach($h in $Hosts){ Snapshot $h "post" }

Log "DONE. Evidence bundle: $Root"

