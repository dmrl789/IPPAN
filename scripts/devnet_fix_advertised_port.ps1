param(
  [string]$User="ippan-devnet",
  [string[]]$Hosts=@("188.245.97.41","135.181.145.174","5.223.51.238","178.156.219.107")
)
$ErrorActionPreference="Stop"

function Ssh([string]$h,[string]$cmd){
  $output = & ssh -o StrictHostKeyChecking=accept-new "$User@$h" $cmd 2>&1
  return $output
}

$remote=@'
set -euo pipefail

echo "== HOST: $(hostname) =="
sudo -n systemctl is-active ippan-node >/dev/null

# Determine config path from systemd ExecStart
EXEC="$(sudo -n systemctl show ippan-node -p ExecStart --value || true)"
CFG=""
if echo "$EXEC" | grep -q -- "--config"; then
  CFG="$(echo "$EXEC" | sed -E 's/.*--config[= ]([^ ]+).*/\1/' | tr -d '"')"
fi
if [ -z "$CFG" ] || [ ! -f "$CFG" ]; then
  # common fallback
  if [ -f /etc/ippan/config/node.toml ]; then CFG="/etc/ippan/config/node.toml"; fi
fi
if [ -z "$CFG" ] || [ ! -f "$CFG" ]; then
  echo "ERROR: Could not locate config file. ExecStart=$EXEC"
  exit 2
fi

echo "Config: $CFG"

OK8080=0
OK9000=0
curl -fsS http://127.0.0.1:8080/p2p/peers >/dev/null 2>&1 && OK8080=1 || true
curl -fsS http://127.0.0.1:9000/p2p/peers >/dev/null 2>&1 && OK9000=1 || true

echo "p2p/peers: 8080=$OK8080 9000=$OK9000"

# Only patch if 8080 works and 9000 doesn't (so we don't break an actually-working 9000 deployment)
if [ "$OK8080" -eq 1 ] && [ "$OK9000" -eq 0 ]; then
  TS="$(date -u +%Y%m%dT%H%M%SZ)"
  sudo -n cp -a "$CFG" "$CFG.bak.$TS"
  echo "Backed up to $CFG.bak.$TS"

  # Patch p2p.port to 8080 (idempotent)
  if grep -q '^[[:space:]]*port[[:space:]]*=' "$CFG"; then
    # If there's a [p2p] block, patch within it; otherwise patch the first matching port line conservatively
    # Conservative approach: patch only lines after a [p2p] header until the next [section]
    python3 - <<'PY'
import re,sys
path=sys.argv[1]
with open(path,'r',encoding='utf-8') as f: lines=f.readlines()

out=[]
in_p2p=False
patched=False
for line in lines:
    m=re.match(r'^\s*\[(.+?)\]\s*$', line)
    if m:
        in_p2p=(m.group(1).strip()=="p2p")
    if in_p2p and re.match(r'^\s*port\s*=\s*\d+\s*$', line):
        out.append(re.sub(r'^\s*port\s*=\s*\d+\s*$', 'port = 8080', line))
        patched=True
    else:
        out.append(line)

# If no [p2p] port line existed, add it to [p2p]
if not patched:
    out2=[]
    inserted=False
    for i,line in enumerate(out):
        out2.append(line)
        if not inserted and re.match(r'^\s*\[p2p\]\s*$', line):
            # insert after header
            out2.append('port = 8080\n')
            inserted=True
    out=out2

with open(path,'w',encoding='utf-8') as f: f.writelines(out)
print("patched=", ("yes" if True else "no"))
PY
"$CFG"

  else
    # No port key anywhere: append under [p2p]
    if grep -q '^\s*\[p2p\]\s*$' "$CFG"; then
      sudo -n sh -c "printf '\nport = 8080\n' >> '$CFG'"
    else
      sudo -n sh -c "printf '\n[p2p]\nport = 8080\n' >> '$CFG'"
    fi
  fi

  sudo -n systemctl restart ippan-node
  sleep 2
  curl -fsS http://127.0.0.1:8080/p2p/peers >/dev/null
  echo "PATCH_OK"
else
  echo "SKIP_PATCH"
fi
'@

foreach($h in $Hosts){
  Write-Host "---- $h ----" -ForegroundColor Cyan
  # Run remote bash via ssh - pass script via stdin
  $remote | & ssh -o StrictHostKeyChecking=accept-new "$User@$h" "bash"
}

