#!/usr/bin/env bash
set -euo pipefail

# Collect /status and /consensus/view JSON from a list of nodes into out/verify_<UTCSTAMP>/.
#
# Usage:
#   ./scripts/ops/collect_verify_bundle.sh 188.245.97.41 <NODE2_IP> <NODE3_IP> <NODE4_IP>

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <NODE_IP...>" >&2
  exit 1
fi

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="out/verify_${STAMP}"
mkdir -p "$OUT"
echo "evidence -> $OUT"

for IP in "$@"; do
  echo "== $IP =="
  curl -sS "http://$IP:8080/status" > "$OUT/status_${IP}.json" || true
  curl -sS "http://$IP:8080/consensus/view" > "$OUT/consensus_${IP}.json" || true
done

python3 - <<'PY' "$OUT"
import json, glob, os, sys
out = sys.argv[1]
paths = sorted(glob.glob(os.path.join(out, "status_*.json")))
for p in paths:
    ip=os.path.basename(p).split("_",1)[1].replace(".json","")
    try:
        j=json.load(open(p))
        print(ip,
              "validator_count=", j.get("validator_count"),
              "source=", j.get("validator_source"),
              "peer_count=", j.get("peer_count"),
              "sample=", (j.get("validator_ids_sample") or [])[:4])
    except Exception as e:
        print(ip, "ERR", e)
PY


