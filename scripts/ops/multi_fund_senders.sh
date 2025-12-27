#!/usr/bin/env bash
set -euo pipefail

# Fund generated senders via /dev/fund (requires IPPAN_DEV_MODE=true on the node AND
# /dev/fund only accepts requests from localhost, so use an SSH tunnel RPC).
#
# Usage:
#   RPC="http://127.0.0.1:18080" SENDERS_FILE="out/senders/senders.json" AMOUNT="200000" ./scripts/ops/multi_fund_senders.sh

RPC="${RPC:-}"
if [[ -z "$RPC" ]]; then
  echo "ERROR: RPC env var required (e.g. http://127.0.0.1:18080 tunnel to api1)" >&2
  exit 2
fi

SENDERS_FILE="${SENDERS_FILE:-out/senders/senders.json}"
if [[ ! -f "$SENDERS_FILE" ]]; then
  echo "ERROR: missing senders file: $SENDERS_FILE" >&2
  exit 2
fi

AMOUNT="${AMOUNT:-200000}"

python3 - "$RPC" "$SENDERS_FILE" "$AMOUNT" <<'PY'
import json,sys,urllib.request

rpc = sys.argv[1].rstrip("/")
senders_file = sys.argv[2]
amount = int(sys.argv[3])

with open(senders_file, "r", encoding="utf-8") as f:
    senders = json.load(f)

ok=0
fail=0
for s in senders:
    addr = s.get("address")
    if not addr:
        print("missing address in senders.json entry", file=sys.stderr)
        fail += 1
        continue
    body = json.dumps({"address": addr, "amount": amount}).encode("utf-8")
    req = urllib.request.Request(
        rpc + "/dev/fund",
        data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            if resp.status >= 200 and resp.status < 300:
                ok += 1
            else:
                fail += 1
                print(f"fund {addr} failed status={resp.status}", file=sys.stderr)
    except Exception as e:
        fail += 1
        print(f"fund {addr} errored: {e}", file=sys.stderr)

print("fund_ok:", ok)
print("fund_fail:", fail)
print(f"SUMMARY fund_ok={ok} fund_fail={fail}")
PY


