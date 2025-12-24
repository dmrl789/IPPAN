#!/usr/bin/env bash
set -euo pipefail

# Quick validator visibility diagnostics (no jq).
#
# Usage:
#   ./scripts/ops/diag_validators.sh http://188.245.97.41:8080

RPC="${1:-http://188.245.97.41:8080}"

echo "RPC=$RPC"
echo "--- /status ---"
curl -sS "$RPC/status"; echo
echo "--- /consensus/view ---"
curl -sS "$RPC/consensus/view"; echo
echo "--- quick checks ---"
echo "peer_count:" "$(curl -sS "$RPC/status" | python3 -c 'import sys,json; j=json.load(sys.stdin); print(j.get(\"peer_count\"))')"
echo "validator_count:" "$(curl -sS "$RPC/status" | python3 -c 'import sys,json; j=json.load(sys.stdin); print(j.get(\"validator_count\"))')"
echo "validator_source:" "$(curl -sS "$RPC/status" | python3 -c 'import sys,json; j=json.load(sys.stdin); print(j.get(\"validator_source\"))')"
echo "validator_ids_sample:" "$(curl -sS "$RPC/status" | python3 -c 'import sys,json; j=json.load(sys.stdin); print(j.get(\"validator_ids_sample\"))')"


