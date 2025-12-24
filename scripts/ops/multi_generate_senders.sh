#!/usr/bin/env bash
set -euo pipefail

# Generate N devnet sender keyfiles (insecure plaintext) and write out/senders/senders.json.
#
# Output layout:
#   out/senders/
#     keys/s001.key ...
#     senders.json
#     senders.pretty.json
#
# NOTE: keyfiles are generated with --insecure-plaintext for load testing convenience.
#       Ensure `out/` is gitignored (it is).

N="${1:-10}"
if ! [[ "$N" =~ ^[0-9]+$ ]] || [[ "$N" -le 0 ]]; then
  echo "Usage: $0 <N>   (N must be a positive integer)"
  exit 1
fi

: "${OUT_DIR:=out/senders}"
: "${NETWORK:=devnet}"

KEY_DIR="$OUT_DIR/keys"
mkdir -p "$KEY_DIR"

WALLET_BIN="${WALLET_BIN:-}"
if [[ -z "$WALLET_BIN" ]]; then
  if [[ -x ./target/release/ippan-wallet ]]; then
    WALLET_BIN="./target/release/ippan-wallet"
  else
    WALLET_BIN="cargo run --quiet -p ippan_wallet --bin ippan-wallet --"
  fi
fi

echo "OUT_DIR=$OUT_DIR"
echo "N=$N"
echo "WALLET_BIN=$WALLET_BIN"

tmp_json="$(mktemp)"
python3 - "$N" "$OUT_DIR" > "$tmp_json" <<'PY'
import json, sys, os
n=int(sys.argv[1])
out_dir=sys.argv[2]
items=[]
for i in range(1,n+1):
    items.append({
        "from": f"@s{i:03d}.ipn",
        "pubkey_hex": "",
        "signing_key_file": os.path.join(out_dir, "keys", f"s{i:03d}.key").replace("\\","/"),
        "address": ""
    })
print(json.dumps(items, indent=2))
PY

mv "$tmp_json" "$OUT_DIR/senders.json"

for i in $(seq 1 "$N"); do
  i3="$(printf "%03d" "$i")"
  key="$KEY_DIR/s${i3}.key"
  # generate-key refuses unencrypted keys unless --insecure-plaintext is set.
  # shellcheck disable=SC2086
  $WALLET_BIN --network "$NETWORK" generate-key --out "$key" --force --insecure-plaintext --notes "txload sender s${i3}" >/dev/null

  # shellcheck disable=SC2086
  info="$($WALLET_BIN --network "$NETWORK" show-address --key "$key" --json)"
  pubkey_hex="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["public_key_hex"])' <<<"$info")"
  address="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["address"])' <<<"$info")"

  python3 - "$OUT_DIR/senders.json" "$key" "$pubkey_hex" "$address" <<'PY'
import json, sys
path, key, pub, addr = sys.argv[1:5]
data=json.load(open(path,"r",encoding="utf-8"))
for item in data:
    if item["signing_key_file"] == key:
        item["pubkey_hex"] = pub
        item["address"] = addr
        break
json.dump(data, open(path,"w",encoding="utf-8"), indent=2)
PY
done

python3 -m json.tool "$OUT_DIR/senders.json" > "$OUT_DIR/senders.pretty.json" || true

echo "Wrote:"
echo "  $OUT_DIR/senders.json"
echo "  $OUT_DIR/senders.pretty.json"
echo "  $KEY_DIR/ (keyfiles)"


