#!/usr/bin/env bash
set -euo pipefail

RPC_URL="${RPC_URL:-http://127.0.0.1:18080}"
KEY_PATH="${KEY_PATH:-./tmp/devnet-wallet.key}"
AMOUNT="${AMOUNT:-0.01}"
FEE="${FEE:-0.000001}"
MEMO="${MEMO:-wallet smoke test}"
TO_ADDRESS="${TO_ADDRESS:-auto}"

mkdir -p "$(dirname "$KEY_PATH")"

echo "[1/5] Generating throwaway devnet key at $KEY_PATH"
ippan-wallet --network devnet generate-key \
  --out "$KEY_PATH" \
  --force \
  --insecure-plaintext >/dev/null

ADDRESS=$(ippan-wallet show-address --key "$KEY_PATH" --json | jq -r '.address')
echo "      Derived address: $ADDRESS"

TARGET_ADDRESS="$ADDRESS"
if [[ "$TO_ADDRESS" != "auto" ]]; then
  TARGET_ADDRESS="$TO_ADDRESS"
fi

echo "[2/5] Funding $ADDRESS via ${RPC_URL}/dev/fund"
curl -sS -X POST "${RPC_URL%/}/dev/fund" \
  -H "Content-Type: application/json" \
  -d "{\"address\":\"$ADDRESS\",\"amount\":100000000000000000000000,\"nonce\":0}" \
  | jq .

echo "[3/5] Submitting signed payment -> $TARGET_ADDRESS"
SEND_OUTPUT=$(ippan-wallet --rpc-url "$RPC_URL" send-payment \
  --key "$KEY_PATH" \
  --to "$TARGET_ADDRESS" \
  --amount "$AMOUNT" \
  --fee "$FEE" \
  --memo "$MEMO" \
  --yes)

echo "$SEND_OUTPUT"
TX_HASH=$(grep -Eo 'Tx hash: [0-9a-fA-F]+' <<<"$SEND_OUTPUT" | awk '{print $3}')
if [[ -z "$TX_HASH" ]]; then
  echo "Failed to extract tx hash from wallet output" >&2
  exit 1
fi
echo "[4/5] Payment hash: $TX_HASH"

echo "[5/5] Verifying via ${RPC_URL}/tx/$TX_HASH"
curl -sS "${RPC_URL%/}/tx/$TX_HASH" | jq .

echo "Smoke test complete ✅"
