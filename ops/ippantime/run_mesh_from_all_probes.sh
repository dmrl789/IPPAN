#!/usr/bin/env bash
set -euo pipefail

# Runs probes from:
# - TR-A
# - TR-B
# - optionally the 4 servers too (making a 6-probe mesh)
#
# Requires passwordless SSH keys set up from your operator machine
# OR run this directly from a machine that can ssh to all.

# Resolve base dir robustly (works in repo and when copied to /root/ippantime)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_DIR="$SCRIPT_DIR"  # when copied remote, ops/ippantime is the base

# If running inside a git checkout, prefer repo root
if git rev-parse --show-toplevel >/dev/null 2>&1; then
  REPO_ROOT="$(git rev-parse --show-toplevel)"
  BASE_DIR="$REPO_ROOT/ops/ippantime"
fi

INV="$BASE_DIR/inventory.yaml"
OUTDIR="$BASE_DIR/out"

INV="$INV" python3 - <<'PY'
import yaml
import os
inv=yaml.safe_load(open(os.environ["INV"],"r",encoding="utf-8"))
print("Loaded inventory OK")
PY

# helper to run a command remotely
rssh () {
  local USERHOST="$1"; shift
  ssh -o StrictHostKeyChecking=accept-new -o ConnectTimeout=5 "$USERHOST" "$@"
}

# copy ops/ippantime folder to remote /root/ippantime (idempotent)
rcopy () {
  local USERHOST="$1"
  rsync -az --delete "$BASE_DIR/" "$USERHOST:/root/ippantime/"
}

# run probe on remote
rprobe () {
  local USERHOST="$1"
  local PROBE_NAME="$2"
  rssh "$USERHOST" "cd /root/ippantime && bash run_probe_from_host.sh $PROBE_NAME"
}

# parse inventory (minimal parsing via python to avoid jq)
readarray -t PROBES < <(INV="$INV" python3 - <<'PY'
import os, yaml
inv=yaml.safe_load(open(os.environ["INV"],"r",encoding="utf-8"))
probes=[]
for t in inv.get("threadrippers",[]):
    probes.append((t["name"], f'{t["ssh_user"]}@{t["host"]}'))
if os.environ.get("INCLUDE_SERVER_PROBES","0") == "1":
    for s in inv.get("servers",[]):
        probes.append((s["name"], f'{s["ssh_user"]}@{s["host"]}'))
for name, userhost in probes:
    print(name + " " + userhost)
PY
)

for line in "${PROBES[@]}"; do
  NAME="$(echo "$line" | awk '{print $1}')"
  USERHOST="$(echo "$line" | awk '{print $2}')"
  echo "== PROBE: $NAME ($USERHOST) =="
  rcopy "$USERHOST"
  rssh "$USERHOST" "python3 -c 'import yaml; print(\"PyYAML OK\")'" || true
  rprobe "$USERHOST" "$NAME"
done

echo
echo "== Collect outputs back to local ops/ippantime/out =="
mkdir -p "$OUTDIR"
for line in "${PROBES[@]}"; do
  NAME="$(echo "$line" | awk '{print $1}')"
  USERHOST="$(echo "$line" | awk '{print $2}')"
  rsync -az "$USERHOST:/root/ippantime/out/ippantime_${NAME}.csv" "$OUTDIR/" || true
done

echo "== Analyze =="
if [[ -n "${REPO_ROOT:-}" ]]; then
  (cd "$REPO_ROOT" && python3 "$BASE_DIR/bin/analyze_ippantime.py")
else
  python3 "$BASE_DIR/bin/analyze_ippantime.py"
fi
echo "DONE. See $OUTDIR/REPORT.md"
