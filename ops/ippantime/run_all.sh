#!/usr/bin/env bash
set -euo pipefail

echo "== IPPAN TIME MESH TEST (40 nodes + 2 TR) =="

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

# 1) local sanity: generate targets
python3 -c "import yaml; print('PyYAML OK')"
python3 "$BASE_DIR/bin/gen_targets.py" "$INV" >/dev/null
echo "Targets generation OK"

# 2) run mesh probes remotely and collect
bash "$BASE_DIR/run_mesh_from_all_probes.sh"

# 3) final outputs
echo
echo "Artifacts:"
ls -la "$OUTDIR" || true
echo
echo "Open:"
echo " - $OUTDIR/REPORT.md"
