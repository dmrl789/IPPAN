## DevNet Transaction Bot (nonce-based loop with PID control)

This runbook installs and operates a small “tx loop” bot that submits payments with an **explicit nonce** (aligned to on-chain state), plus simple **start/stop/status** wrappers and a tiny **nonce/balance smoke-check**.

### Assumptions (edit if needed)

- **Bot host**: `88.198.26.37`
- **DevNet RPC**: `http://188.245.97.41:8080`
- **Bot wallet (keyfile)**:
  - `KEY_FILE=/root/ippan/keys/bot_wallet.key`
  - `PASSWORD_FILE=/root/ippan/keys/bot_wallet.pw` (optional if key is plaintext)
- **Loop script (on bot)**: `/root/devnet_tx_loop_nonce.sh`
- **PID file**: `/var/run/ippan-tx-loop.pid`
- **Log file**: `/var/log/ippan-tx-loop.log`

> Note: this repo’s `ippan-wallet` is **keyfile-based** (`--key`, `--password-file`, `--rpc-url`). If your bot host uses a different wallet binary (e.g., `--wallet-name`), adapt the install commands accordingly.

---

## 1) Install/replace the loop script (with PID + correct nonce behavior)

From your laptop (WSL), paste:

```bash
ssh root@88.198.26.37 'cat > /root/devnet_tx_loop_nonce.sh' << "EOF"
#!/usr/bin/env bash
set -euo pipefail

RPC_URL="${RPC_URL:-http://188.245.97.41:8080}"
TO="${TO:-12X6G6FooBZbBZi7t5CdEWJmZCn8L6GquxthxFQwM9PFzb93RWD}"

KEY_FILE="${KEY_FILE:-/root/ippan/keys/bot_wallet.key}"
PASSWORD_FILE="${PASSWORD_FILE:-/root/ippan/keys/bot_wallet.pw}"

AMOUNT_ATOMIC="${AMOUNT_ATOMIC:-1000000}"
FEE_ATOMIC="${FEE_ATOMIC:-}"
SLEEP_SEC="${SLEEP_SEC:-0.2}"

PID_FILE="${PID_FILE:-/var/run/ippan-tx-loop.pid}"

need_cmd() { command -v "$1" >/dev/null 2>&1; }

mkdir -p "$(dirname "$PID_FILE")"

if [[ -f "$PID_FILE" ]]; then
  old_pid="$(cat "$PID_FILE" 2>/dev/null || true)"
  if [[ -n "${old_pid}" ]] && kill -0 "${old_pid}" 2>/dev/null; then
    echo "Loop already running with PID ${old_pid}"
    exit 1
  fi
  rm -f "$PID_FILE"
fi

if ! need_cmd ippan-wallet; then
  echo "ERROR: ippan-wallet not found in PATH" >&2
  exit 127
fi
if ! need_cmd curl; then
  echo "ERROR: curl not found" >&2
  exit 127
fi
if ! need_cmd jq; then
  echo "ERROR: jq is required for JSON parsing" >&2
  exit 127
fi

echo "Starting nonce-based tx loop to $TO via $RPC_URL at $(date -Is)"

PW_ARGS=()
if [[ -n "${PASSWORD_FILE}" && -f "${PASSWORD_FILE}" ]]; then
  PW_ARGS=(--password-file "$PASSWORD_FILE")
fi

ADDR_JSON="$(ippan-wallet show-address --key "$KEY_FILE" "${PW_ARGS[@]}" --json)"
FROM_ADDR="$(jq -r '.address' <<<"$ADDR_JSON")"
FROM_PUB_HEX="$(jq -r '.public_key_hex' <<<"$ADDR_JSON")"

ACCOUNT_JSON="$(curl -fsS "${RPC_URL%/}/account/${FROM_PUB_HEX}")"
CURRENT_NONCE="$(jq -r '.nonce' <<<"$ACCOUNT_JSON")"
NEXT_NONCE=$((CURRENT_NONCE + 1))

echo $$ > "$PID_FILE"
trap 'rm -f "$PID_FILE"; exit 0' INT TERM EXIT

WALLET_ARGS=(--rpc-url "$RPC_URL" send-payment --key "$KEY_FILE" "${PW_ARGS[@]}" --to "$TO" --amount-atomic "$AMOUNT_ATOMIC" --yes)
if [[ -n "${FEE_ATOMIC}" ]]; then
  WALLET_ARGS+=(--fee-atomic "$FEE_ATOMIC")
fi

while true; do
  TS="$(date -Is)"
  # Important: on error, retry the SAME nonce (never skip).
  if ippan-wallet "${WALLET_ARGS[@]}" --nonce "$NEXT_NONCE" --memo "txloop-nonce-$NEXT_NONCE"; then
    echo "$TS OK nonce=$NEXT_NONCE amount_atomic=$AMOUNT_ATOMIC to=$TO"
    NEXT_NONCE=$((NEXT_NONCE + 1))
  else
    echo "$TS ERROR nonce=$NEXT_NONCE (will retry same nonce)" >&2
    sleep 1
  fi
  sleep "$SLEEP_SEC"
done
EOF

ssh root@88.198.26.37 'chmod +x /root/devnet_tx_loop_nonce.sh && ls -l /root/devnet_tx_loop_nonce.sh'
```

---

## 2) Install helper scripts: start / stop / status

```bash
ssh root@88.198.26.37 'cat > /usr/local/bin/ippan-tx-loop-start' << "EOF"
#!/usr/bin/env bash
set -euo pipefail

LOOP_SCRIPT="${LOOP_SCRIPT:-/root/devnet_tx_loop_nonce.sh}"
PID_FILE="${PID_FILE:-/var/run/ippan-tx-loop.pid}"
LOG_FILE="${LOG_FILE:-/var/log/ippan-tx-loop.log}"

mkdir -p "$(dirname "$PID_FILE")"
mkdir -p "$(dirname "$LOG_FILE")"
touch "$LOG_FILE"

if [[ -f "$PID_FILE" ]]; then
  PID="$(cat "$PID_FILE" 2>/dev/null || true)"
  if [[ -n "$PID" ]] && kill -0 "$PID" 2>/dev/null; then
    echo "tx loop already running (PID $PID)"
    exit 0
  fi
fi

if [[ ! -x "$LOOP_SCRIPT" ]]; then
  echo "ERROR: loop script not found/executable at $LOOP_SCRIPT" >&2
  exit 1
fi

nohup "$LOOP_SCRIPT" >> "$LOG_FILE" 2>&1 &
sleep 0.5

if [[ -f "$PID_FILE" ]]; then
  PID="$(cat "$PID_FILE" 2>/dev/null || true)"
  if [[ -n "$PID" ]] && kill -0 "$PID" 2>/dev/null; then
    echo "Started tx loop with PID $PID"
    exit 0
  fi
fi

echo "ERROR: tx loop did not start (missing or invalid PID file: $PID_FILE)" >&2
exit 1
EOF

ssh root@88.198.26.37 'cat > /usr/local/bin/ippan-tx-loop-stop' << "EOF"
#!/usr/bin/env bash
set -euo pipefail

PID_FILE="${PID_FILE:-/var/run/ippan-tx-loop.pid}"
TIMEOUT_SEC="${TIMEOUT_SEC:-10}"

if [[ ! -f "$PID_FILE" ]]; then
  echo "No PID file; tx loop not running?"
  exit 0
fi

PID="$(cat "$PID_FILE" 2>/dev/null || true)"
if [[ -z "$PID" ]]; then
  echo "Stale PID file (empty); cleaning"
  rm -f "$PID_FILE"
  exit 0
fi

if ! kill -0 "$PID" 2>/dev/null; then
  echo "PID $PID not running; cleaning stale PID file"
  rm -f "$PID_FILE"
  exit 0
fi

kill "$PID" 2>/dev/null || true

deadline=$(( $(date +%s) + TIMEOUT_SEC ))
while kill -0 "$PID" 2>/dev/null; do
  if [[ "$(date +%s)" -ge "$deadline" ]]; then
    echo "Process still running after ${TIMEOUT_SEC}s; sending SIGKILL"
    kill -9 "$PID" 2>/dev/null || true
    break
  fi
  sleep 0.2
done

rm -f "$PID_FILE"
echo "Stopped tx loop (PID $PID)"
EOF

ssh root@88.198.26.37 'cat > /usr/local/bin/ippan-tx-loop-status' << "EOF"
#!/usr/bin/env bash
set -euo pipefail

PID_FILE="${PID_FILE:-/var/run/ippan-tx-loop.pid}"

if [[ ! -f "$PID_FILE" ]]; then
  echo "tx loop: not running (no PID file)"
  exit 0
fi

PID="$(cat "$PID_FILE" 2>/dev/null || true)"
if [[ -z "$PID" ]]; then
  echo "tx loop: PID file present but empty"
  exit 0
fi

if kill -0 "$PID" 2>/dev/null; then
  echo "tx loop: running (PID $PID)"
else
  echo "tx loop: PID file present but process not running (PID $PID)"
fi
EOF

ssh root@88.198.26.37 'chmod +x /usr/local/bin/ippan-tx-loop-* && ls -l /usr/local/bin/ippan-tx-loop-*'
```

---

## 3) Smoke-test start / status / stop

```bash
ssh root@88.198.26.37 '
  ippan-tx-loop-start
  ippan-tx-loop-status
  sleep 2
  tail -n 5 /var/log/ippan-tx-loop.log
'
```

Stop:

```bash
ssh root@88.198.26.37 '
  ippan-tx-loop-stop
  ippan-tx-loop-status
'
```

---

## 4) Tiny DevNet health + nonce/balance check (local helper)

Use the repo helper script:

- `scripts/devnet/check_bot_nonce.sh`

Example:

```bash
RPC="http://188.245.97.41:8080" \
BOT_ADDR="1GSXnrmfKjH8U1vQVjqL2GGyZ8WD7G9zMVJCSNA4eNVrCLKVtB" \
scripts/devnet/check_bot_nonce.sh
```

This prints:

- `GET /status`
- `GET /account/<hex>` (the script converts Base58Check `BOT_ADDR` into the hex form the RPC expects)

---

## 5) Window2 closeout (after ~24h): datasets → model → (optional) activate

The closeout script lives at:

- `docs/ops/devnet-window2-after-24h.sh`

It will:

- Stop the bot loop (unless `--check`)
- Verify datasets on all validators (`/var/lib/ippan/ai_datasets`)
- Pull datasets to laptop under `ai_assets/datasets/devnet_window2/<validator_ip>/`
- Merge + train `ai_assets/models/devnet_dlc_window2/model.json`
- Compute canonical hash into `ai_assets/models/devnet_dlc_window2/model.hash`
- Upload artifacts to validators under `/opt/ippan/ai/models/devnet_dlc_window2/`
- Optionally patch `dlc.toml` + restart nodes + verify `/ai/status`
- Append a run report to `docs/ops/devnet-tx-bot-window2.log`

### Safe dry-run (no stop, no training, no deploy)

```bash
./docs/ops/devnet-window2-after-24h.sh --check
```

### Full run (train + upload, but do NOT activate)

```bash
./docs/ops/devnet-window2-after-24h.sh
```

### Full run + activate new model (patch config + restart)

```bash
./docs/ops/devnet-window2-after-24h.sh --activate
```


