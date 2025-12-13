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
  $result = & ssh.exe $sshTarget $cmd 2>&1
  return $result
}

$ts = Get-Date -Format "yyyyMMdd_HHmmss"
$Root = "tmp/devnet/p2p_normalize_$ts"
New-Item -ItemType Directory -Force -Path $Root | Out-Null

function Log([string]$msg){
  $line = "[{0}] {1}" -f (Get-Date -Format "s"), $msg
  $line | Tee-Object -FilePath (Join-Path $Root "run.log") -Append | Out-Host
}

function RemoteConfigPath([string]$h){
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
echo "$CFG"
'@
  $bashCmd = $bashCmd.Replace("`r","").Replace('"','\"')
  $out = Ssh $h ("bash -lc " + '"' + $bashCmd + '"')
  $cfg = ($out | Select-Object -Last 1).Trim()
  if (-not $cfg) { return "" }
  return $cfg
}

function Snapshot([string]$h,[string]$tag){
  $d = Join-Path $Root ($h.Replace(".","_"))
  New-Item -ItemType Directory -Force -Path $d | Out-Null

  $verifyScript = @'
echo HOST:; hostname; echo UTC:; date -u; echo ---; echo ACTIVE:; sudo -n systemctl is-active ippan-node || true; echo ---; echo STATUS:; curl -fsS http://127.0.0.1:8080/status || true; echo; echo ---; echo PEERS:; curl -fsS http://127.0.0.1:8080/p2p/peers || true; echo
'@
  $verifyScript = $verifyScript.Replace("`r","").Replace('"','\"')
  Ssh $h ("bash -lc " + '"' + $verifyScript + '"') | Set-Content -Encoding UTF8 (Join-Path $d "$tag`_verify.txt")

  $listenersScript = @'
echo LISTENERS:; ss -lntp 2>/dev/null || true; echo ---; ss -lnup 2>/dev/null || true
'@
  $listenersScript = $listenersScript.Replace("`r","").Replace('"','\"')
  Ssh $h ("bash -lc " + '"' + $listenersScript + '"') | Set-Content -Encoding UTF8 (Join-Path $d "$tag`_listeners.txt")

  $journalScript = "sudo -n journalctl -u ippan-node -n 160 --no-pager || true"
  $journalScript = $journalScript.Replace("'","'\''")
  Ssh $h ("bash -lc '" + $journalScript + "'") | Set-Content -Encoding UTF8 (Join-Path $d "$tag`_journal_tail.txt")

  $cfg = RemoteConfigPath $h
  if ($cfg) {
    $cfgScript = @'
echo CFG_PATH:
echo CFG_PATH_VALUE
echo ---
sed -n '1,220p' 'CFG_PATH_VALUE'
'@
    $cfgScript = $cfgScript.Replace("`r","").Replace("CFG_PATH_VALUE", $cfg).Replace("'","'\''")
    Ssh $h ("bash -lc " + "'" + $cfgScript + "'") | Set-Content -Encoding UTF8 (Join-Path $d "$tag`_config_head.txt")
  } else {
    "CFG_PATH:EMPTY" | Set-Content -Encoding UTF8 (Join-Path $d "$tag`_config_head.txt")
  }
}

Log "STEP 1: PRE snapshot"
foreach($h in $Hosts){ Snapshot $h "pre" }

Log "STEP 2: Patch configs (normalize P2P listen/advertise = :9000)."

$adv = @{
  $Node1 = "http://$Node1:9000"
  $Node2 = "http://$Node2:9000"
  $Node3 = "http://$Node3:9000"
  $Node4 = "http://$Node4:9000"
}

$patchPy = @'
import sys,re,datetime
path=sys.argv[1]
adv=sys.argv[2]
want_port="9000"

txt=open(path,"r",encoding="utf-8").read().splitlines(True)

keys_port = [
  r'^\s*p2p_port\s*=\s*\d+\s*$',
  r'^\s*p2p_listen_port\s*=\s*\d+\s*$',
  r'^\s*listen_p2p_port\s*=\s*\d+\s*$',
]
keys_adv = [
  r'^\s*advertised_address\s*=\s*".*"\s*$',
  r'^\s*p2p_advertised_address\s*=\s*".*"\s*$',
]

found_port=False
found_adv=False

out=[]
for ln in txt:
  done=False
  for pat in keys_port:
    if re.match(pat, ln):
      k=ln.split("=",1)[0].strip()
      out.append(f"{k} = {want_port}\n")
      found_port=True
      done=True
      break
  if done:
    continue

  for pat in keys_adv:
    if re.match(pat, ln):
      k=ln.split("=",1)[0].strip()
      out.append(f'{k} = "{adv}"\n')
      found_adv=True
      done=True
      break
  if done:
    continue

  out.append(ln)

if not (found_port or found_adv):
  print("UNKNOWN_KEYS")
  sys.exit(3)

# If we found one but not the other, add the missing setting at EOF (safe).
if not found_port:
  out.append("\n# added by p2p_normalize\np2p_port = 9000\n")
if not found_adv:
  out.append(f'\n# added by p2p_normalize\nadvertised_address = "{adv}"\n')

bak = path + ".bak." + datetime.datetime.utcnow().strftime("%Y%m%dT%H%M%SZ")
open(bak,"w",encoding="utf-8").write("".join(txt))
open(path,"w",encoding="utf-8").write("".join(out))
print("OK")
'@

foreach($h in $Hosts){
  $cfg = RemoteConfigPath $h
  Log "Patch $h cfg=$cfg adv=$($adv[$h])"
  if (-not $cfg) {
    ("ERROR: config not found on {0}" -f $h) | Tee-Object -FilePath (Join-Path $Root "patch_errors.log") -Append | Out-Host
    continue
  }

  $patchScript = @'
set -e
CFG="$1"
ADV="$2"
TS=$(date -u +%Y%m%dT%H%M%SZ)
sudo -n cp -a "$CFG" "$CFG.prepatch.$TS"
python3 - "$CFG" "$ADV" <<'PY'
import sys,re,datetime
path=sys.argv[1]
adv=sys.argv[2]
want_port="9000"

txt=open(path,"r",encoding="utf-8").read().splitlines(True)

keys_port = [
  r'^\s*p2p_port\s*=\s*\d+\s*$',
  r'^\s*p2p_listen_port\s*=\s*\d+\s*$',
  r'^\s*listen_p2p_port\s*=\s*\d+\s*$',
]
keys_adv = [
  r'^\s*advertised_address\s*=\s*".*"\s*$',
  r'^\s*p2p_advertised_address\s*=\s*".*"\s*$',
]

found_port=False
found_adv=False

out=[]
for ln in txt:
  done=False
  for pat in keys_port:
    if re.match(pat, ln):
      k=ln.split("=",1)[0].strip()
      out.append(f"{k} = {want_port}\n")
      found_port=True
      done=True
      break
  if done:
    continue

  for pat in keys_adv:
    if re.match(pat, ln):
      k=ln.split("=",1)[0].strip()
      out.append(f'{k} = "{adv}"\n')
      found_adv=True
      done=True
      break
  if done:
    continue

  out.append(ln)

if not (found_port or found_adv):
  print("UNKNOWN_KEYS")
  sys.exit(3)

# If we found one but not the other, add the missing setting at EOF (safe).
if not found_port:
  out.append("\n# added by p2p_normalize\np2p_port = 9000\n")
if not found_adv:
  out.append(f'\n# added by p2p_normalize\nadvertised_address = "{adv}"\n')

bak = path + ".bak." + datetime.datetime.utcnow().strftime("%Y%m%dT%H%M%SZ")
open(bak,"w",encoding="utf-8").write("".join(txt))
open(path,"w",encoding="utf-8").write("".join(out))
print("OK")
PY
echo "---"
grep -nE '(^\s*p2p_|^\s*advertised_)' "$CFG" || true
'@
  $patchScript = $patchScript.Replace("`r","").Replace('"','\"')
  $escapedCfg = $cfg.Replace('"','\"')
  $escapedAdv = $adv[$h].Replace('"','\"')
  $fullCmd = $patchScript + " " + $escapedCfg + " " + $escapedAdv
  $out = Ssh $h ("bash -lc " + '"' + $fullCmd + '"')
  $out | Set-Content -Encoding UTF8 (Join-Path $Root ("patch_{0}.log" -f $h.Replace(".","_")))
}

Log "STEP 3: Restart order node2,node3,node4,node1 (single pass, no waits)."
$restartCmd = "bash -lc 'sudo -n systemctl restart ippan-node'"
Ssh $Node2 $restartCmd | Out-Null
Ssh $Node3 $restartCmd | Out-Null
Ssh $Node4 $restartCmd | Out-Null
Ssh $Node1 $restartCmd | Out-Null

Log "STEP 4: POST snapshot"
foreach($h in $Hosts){ Snapshot $h "post" }

Log "DONE. Evidence bundle: $Root"

