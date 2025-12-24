#!/usr/bin/env bash
set -euo pipefail

# Generates a sender + receiver wallet keyfile for txload testing.
# Output is written under ./out/txload/keys (gitignored).

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${OUT_DIR:-out/txload/keys}"
SENDER_KEYFILE="${SENDER_KEYFILE:-$OUT_DIR/sender.key}"
RECEIVER_KEYFILE="${RECEIVER_KEYFILE:-$OUT_DIR/receiver.key}"

WALLET_CMD="${IPPAN_WALLET_CMD:-cargo run -q -p ippan_wallet --bin ippan-wallet --}"

mkdir -p "$OUT_DIR"

echo "Generating sender keyfile: $SENDER_KEYFILE"
$WALLET_CMD generate-key --out "$SENDER_KEYFILE" --force --insecure-plaintext

echo "Generating receiver keyfile: $RECEIVER_KEYFILE"
$WALLET_CMD generate-key --out "$RECEIVER_KEYFILE" --force --insecure-plaintext

SENDER_ADDR="$(python3 - <<'PY'
import json,sys
print(json.load(open(sys.argv[1]))["address"])
PY
"$SENDER_KEYFILE")"

RECEIVER_ADDR="$(python3 - <<'PY'
import json,sys
print(json.load(open(sys.argv[1]))["address"])
PY
"$RECEIVER_KEYFILE")"

echo ""
echo "Export these env vars:"
echo "  export IPPAN_SENDER_KEY=\"$SENDER_KEYFILE\""
echo "  export IPPAN_RECEIVER_ADDR=\"$RECEIVER_ADDR\""
echo ""
echo "Sender address:   $SENDER_ADDR"
echo "Receiver address: $RECEIVER_ADDR"


