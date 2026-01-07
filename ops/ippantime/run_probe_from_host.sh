#!/usr/bin/env bash
set -euo pipefail

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
BINDIR="$BASE_DIR/bin"

PROBE_NAME="${1:?usage: run_probe_from_host.sh <probe-name>}"
python3 -c "import sys; print(sys.version)"

# Ensure deps (PyYAML is needed for target generation)
python3 - <<'PY'
import importlib.util, sys
ok = importlib.util.find_spec("yaml") is not None
print("PyYAML:", "OK" if ok else "MISSING")
sys.exit(0 if ok else 1)
PY

TARGETS_JSON="$(python3 "$BINDIR/gen_targets.py" "$INV")"

STATUS_PATH="$(python3 - "$INV" <<'PY'
import yaml, sys
inv=yaml.safe_load(open(sys.argv[1],"r",encoding="utf-8"))
print(inv["http"]["status_path"])
PY
)"

IPPAN_FIELD="$(python3 - "$INV" <<'PY'
import yaml, sys
inv=yaml.safe_load(open(sys.argv[1],"r",encoding="utf-8"))
print(inv["http"]["ippan_time_field"])
PY
)"

SAMPLES="$(python3 - "$INV" <<'PY'
import yaml, sys
inv=yaml.safe_load(open(sys.argv[1],"r",encoding="utf-8"))
print(inv["probe"]["samples"])
PY
)"

INTERVAL_MS="$(python3 - "$INV" <<'PY'
import yaml, sys
inv=yaml.safe_load(open(sys.argv[1],"r",encoding="utf-8"))
print(inv["probe"]["interval_ms"])
PY
)"

TIMEOUT="$(python3 - "$INV" <<'PY'
import yaml, sys
inv=yaml.safe_load(open(sys.argv[1],"r",encoding="utf-8"))
print(inv["http"]["timeout_sec"])
PY
)"

if ! command -v curl >/dev/null 2>&1; then
  echo "ERROR: curl is required for readiness gate but was not found in PATH" >&2
  exit 1
fi

echo "== Readiness gate: waiting for '$IPPAN_FIELD' on all targets (<=120s) =="
READY_DEADLINE=$((SECONDS + 120))

# Extract target base URLs from TARGETS_JSON
readarray -t TARGET_URLS < <(
  python3 -c 'import sys,json; targets=json.load(sys.stdin); [print(t["url"]) for t in targets]' <<<"$TARGETS_JSON"
)

while true; do
  MISSING=()
  for base_url in "${TARGET_URLS[@]}"; do
    status_url="${base_url%/}${STATUS_PATH}"
    body="$(curl -fsS --max-time "$TIMEOUT" "$status_url" 2>/dev/null || true)"
    if [[ -z "$body" ]]; then
      MISSING+=("$status_url")
      continue
    fi
    if ! python3 -c 'import sys,json; field=sys.argv[1]; payload=json.load(sys.stdin); v=payload.get(field,None); sys.exit(0 if v is not None else 1)' "$IPPAN_FIELD" <<<"$body"; then
      MISSING+=("$status_url")
    fi
  done

  if ((${#MISSING[@]} == 0)); then
    echo "Readiness OK: all targets expose '$IPPAN_FIELD'"
    break
  fi

  if (( SECONDS >= READY_DEADLINE )); then
    echo "ERROR: readiness gate timed out. Missing '$IPPAN_FIELD' from these endpoints:" >&2
    printf '%s\n' "${MISSING[@]}" >&2
    exit 1
  fi

  sleep 2
done

mkdir -p "$OUTDIR"

OUT="$OUTDIR/ippantime_${PROBE_NAME}.csv"

python3 "$BINDIR/ippantime_probe.py" \
  --targets-json "$TARGETS_JSON" \
  --status-path "$STATUS_PATH" \
  --ippan-field "$IPPAN_FIELD" \
  --samples "$SAMPLES" \
  --interval-ms "$INTERVAL_MS" \
  --timeout "$TIMEOUT" \
  --probe-name "$PROBE_NAME" \
  --out "$OUT"

echo "OK: $OUT"
