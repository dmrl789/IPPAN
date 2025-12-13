param(
  [string]$User="ippan-devnet",
  [string]$Node1="188.245.97.41"
)

$patchScript = @'
set -euo pipefail
CFG="/etc/ippan/config/node.toml"
TS="$(date -u +%Y%m%dT%H%M%SZ)"
sudo -n cp -a "$CFG" "$CFG.bak.$TS"
python3 - "$CFG" <<'PY'
import sys,re
path=sys.argv[1]
lines=open(path,'r',encoding='utf-8').read().splitlines(True)
out=[]
in_p2p=False
found_port=False
for ln in lines:
    if ln.strip().startswith("[p2p]"):
        in_p2p=True
        out.append(ln)
    elif ln.strip().startswith("["):
        in_p2p=False
        out.append(ln)
    elif in_p2p and re.match(r'^\s*port\s*=\s*.*', ln):
        out.append("port = 8080\n")
        found_port=True
    else:
        out.append(ln)
if not found_port:
    # Find [p2p] section and add port after it
    for i,ln in enumerate(out):
        if "[p2p]" in ln:
            out.insert(i+1, "port = 8080\n")
            break
open(path,'w',encoding='utf-8').write(''.join(out))
print("OK: P2P port set to 8080")
PY
echo "CFG_NOW:"
grep -A 5 "\[p2p\]" "$CFG" || true
'@

$patchScript = $patchScript.Replace("`r","").Replace('"','\"')
$cmd = "bash -lc " + '"' + $patchScript + '"'
$result = & ssh.exe "${User}@${Node1}" $cmd 2>&1
$result | Out-Host

Write-Host "`nRestarting node1..."
& ssh.exe "${User}@${Node1}" "bash -lc 'sudo -n systemctl restart ippan-node'"

Write-Host "`nWaiting 5 seconds..."
Start-Sleep -Seconds 5

Write-Host "`nChecking node1 status:"
& ssh.exe "${User}@${Node1}" "bash -lc 'curl -fsS http://127.0.0.1:8080/status | python3 -m json.tool'"

Write-Host "`nChecking node1 peers:"
& ssh.exe "${User}@${Node1}" "bash -lc 'curl -fsS http://127.0.0.1:8080/p2p/peers | python3 -m json.tool'"

