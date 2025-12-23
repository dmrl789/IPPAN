#!/usr/bin/env bash
set -euo pipefail

# DevNet Window2 Live Monitor (1-minute heartbeat)
#
# Prints a compact view showing:
# - bot loop alive + PID (via bot host)
# - bot nonce delta (via /account/<hex>)
# - network deltas (peer_count, round, blocks_proposed/verified, rounds_active)
# - dataset exporter freshness (last_ts_utc + last_age_seconds)
# - dataset files count/size on each validator (/var/lib/ippan/ai_datasets)
#
# No secrets printed.

RPC_HOSTS=(
  "188.245.97.41"
  "135.181.145.174"
  "5.223.51.238"
  "178.156.219.107"
)

BOT_HOST="${BOT_HOST:-88.198.26.37}"
RPC_PRIMARY="${RPC_PRIMARY:-http://188.245.97.41:8080}"

# Addresses (Base58). Used for display + optional Base58->hex conversion.
BOT_ADDR="${BOT_ADDR:-1GSXnrmfKjH8U1vQVjqL2GGyZ8WD7G9zMVJCSNA4eNVrCLKVtB}"
USER_ADDR="${USER_ADDR:-12X6G6FooBZbBZi7t5CdEWJmZCn8L6GquxthxFQwM9PFzb93RWD}"

# If your /account endpoint expects hex pubkey, these will be used.
# If unset, we will attempt to derive them from BOT_ADDR/USER_ADDR (Base58Check v0 + 32-byte pubkey).
BOT_HEX="${BOT_HEX:-}"
USER_HEX="${USER_HEX:-}"

SLEEP_SEC="${SLEEP_SEC:-60}"

SSH_OPTS="${SSH_OPTS:--o BatchMode=yes -o StrictHostKeyChecking=accept-new -o ConnectTimeout=8 -o ServerAliveInterval=2 -o ServerAliveCountMax=2}"

need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing required command: $1" >&2; exit 1; }; }
need curl
need ssh
need python3
need date

ts() { date -Is; }

CURL_OPTS=(
  --silent
  --fail
  --connect-timeout "${CURL_CONNECT_TIMEOUT:-2}"
  --max-time "${CURL_MAX_TIME:-4}"
)

py_get() {
  # Usage: echo "$json" | py_get 'path.to.key' 'default'
  local path="${1:?path}"
  local default="${2:-?}"
  python3 -c "import sys, json
try:
  d=json.load(sys.stdin)
except Exception:
  print('$default'); raise SystemExit(0)
p='$path'.split('.')
cur=d
try:
  for k in p:
    if not k:
      continue
    if isinstance(cur, list):
      k=int(k)
    cur=cur[k] if isinstance(cur,(list,tuple)) else cur.get(k)
  if cur is None:
    raise KeyError()
  print(cur)
except Exception:
  print('$default')
" 2>/dev/null
}

py_first_validator_metric() {
  # Usage: echo "$status_json" | py_first_validator_metric 'blocks_proposed' 'default'
  local field="${1:?field}"
  local default="${2:-?}"
  python3 -c "import sys, json
try:
  d=json.load(sys.stdin)
except Exception:
  print('$default'); raise SystemExit(0)
try:
  vals=(d.get('consensus') or {}).get('validators') or {}
  first=next(iter(vals.values()))
  v=(first or {}).get('$field', None)
  print(v if v is not None else '$default')
except Exception:
  print('$default')
" 2>/dev/null
}

addr_to_hex() {
  local addr="$1"
  addr="$(echo -n "$addr" | tr -d '\r\n' | xargs)"
  if [[ -z "$addr" ]]; then
    echo ""
    return 0
  fi
  # If already a 64-hex string, pass through.
  if [[ "$addr" =~ ^[0-9a-fA-F]{64}$ ]]; then
    echo "${addr,,}"
    return 0
  fi

  python3 -c "import sys, hashlib
ALPHABET='123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'
INDEX={c:i for i,c in enumerate(ALPHABET)}
def b58decode(s:str)->bytes:
  n=0
  for ch in s:
    if ch not in INDEX: raise ValueError('invalid base58')
    n=n*58+INDEX[ch]
  h=n.to_bytes((n.bit_length()+7)//8,'big') if n else b''
  pad=0
  for ch in s:
    if ch=='1': pad+=1
    else: break
  return b'\\x00'*pad+h
def checksum(data:bytes)->bytes:
  return hashlib.sha256(hashlib.sha256(data).digest()).digest()[:4]
addr=sys.argv[1].strip()
raw=b58decode(addr)
if len(raw)!=1+32+4: raise SystemExit(1)
ver=raw[0]
payload=raw[:33]
chk=raw[33:]
if ver!=0: raise SystemExit(1)
if chk!=checksum(payload): raise SystemExit(1)
pub=raw[1:33]
print(pub.hex())" "$addr" 2>/dev/null || true
}

get_status() {
  local host="$1"
  curl "${CURL_OPTS[@]}" "http://$host:8080/status" 2>/dev/null || echo "{}"
}

get_ai_status() {
  local host="$1"
  curl "${CURL_OPTS[@]}" "http://$host:8080/ai/status" 2>/dev/null || echo "{}"
}

get_account() {
  local host="$1"
  local hex="$2"
  if [[ -z "$hex" ]]; then
    echo "{}"
  else
    curl "${CURL_OPTS[@]}" "http://$host:8080/account/$hex" 2>/dev/null || echo "{}"
  fi
}

bot_loop_status_line() {
  ssh $SSH_OPTS root@"$BOT_HOST" '
    set -euo pipefail
    if command -v ippan-tx-loop-status >/dev/null 2>&1; then
      s="$(ippan-tx-loop-status 2>/dev/null || true)"
      pid=""
      if [[ -f /var/run/ippan-tx-loop.pid ]]; then
        pid="$(cat /var/run/ippan-tx-loop.pid 2>/dev/null || true)"
      fi
      echo "$s pid_file=${pid:-none}"
    else
      echo "ippan-tx-loop-status missing"
      pgrep -a devnet_tx_loop_nonce.sh || true
    fi
  ' 2>/dev/null || echo "bot_ssh_error=1"
}

dataset_summary_line() {
  local host="$1"
  ssh $SSH_OPTS root@"$host" '
    set -euo pipefail
    DIR="/var/lib/ippan/ai_datasets"
    if [[ -d "$DIR" ]]; then
      c="$(ls -1 "$DIR"/devnet_dataset_*.csv.gz 2>/dev/null | wc -l | tr -d " ")"
      s="$(du -sh "$DIR" 2>/dev/null | awk "{print \$1}")"
      newest="$(ls -1t "$DIR"/devnet_dataset_*.csv.gz 2>/dev/null | head -n 1 || true)"
      echo "datasets_count=$c datasets_size=${s:-?} newest=${newest:-none}"
    else
      echo "datasets_count=0 datasets_size=0 newest=none"
    fi
  ' 2>/dev/null || echo "datasets_error=1"
}

if [[ -z "$BOT_HEX" ]]; then
  BOT_HEX="$(addr_to_hex "$BOT_ADDR")"
fi
if [[ -z "$USER_HEX" ]]; then
  USER_HEX="$(addr_to_hex "$USER_ADDR")"
fi

if [[ -z "$BOT_HEX" || -z "$USER_HEX" ]]; then
  echo "NOTE: could not auto-convert Base58→hex; account nonce/balance lines will be skipped."
  echo "      You can set BOT_HEX and USER_HEX env vars to enable account polling."
  echo
fi

echo "=== DevNet Window2 Live Monitor ==="
echo "RPC_PRIMARY=$RPC_PRIMARY"
echo "BOT_HOST=$BOT_HOST"
echo "BOT_ADDR=$BOT_ADDR"
echo "BOT_HEX=${BOT_HEX:-<unset>}"
echo "USER_HEX=${USER_HEX:-<unset>}"
echo "SLEEP_SEC=$SLEEP_SEC"
echo

prev_blocks=""
prev_verified=""
prev_round=""
prev_bot_nonce=""
prev_user_bal=""

while true; do
  now="$(ts)"
  echo "---- $now ----"

  # Bot loop
  echo "[BOT] $(bot_loop_status_line | tr '\n' ' ' | sed 's/  */ /g')"

  # Primary /status (net + exporter freshness)
  st="$(get_status "188.245.97.41")"

  peer_count="$(printf '%s' "$st" | py_get 'peer_count' '?')"
  round="$(printf '%s' "$st" | py_get 'consensus.round' '?')"
  blocks_proposed="$(printf '%s' "$st" | py_first_validator_metric 'blocks_proposed' '?')"
  blocks_verified="$(printf '%s' "$st" | py_first_validator_metric 'blocks_verified' '?')"
  rounds_active="$(printf '%s' "$st" | py_first_validator_metric 'rounds_active' '?')"
  de_enabled="$(printf '%s' "$st" | py_get 'dataset_export.enabled' '?')"
  de_age="$(printf '%s' "$st" | py_get 'dataset_export.last_age_seconds' '?')"
  de_ts="$(printf '%s' "$st" | py_get 'dataset_export.last_ts_utc' '?')"

  # Deltas
  d_round="?"
  d_blocks="?"
  d_verified="?"
  if [[ -n "${prev_round}" && "${round:-}" =~ ^[0-9]+$ && "${prev_round}" =~ ^[0-9]+$ ]]; then d_round=$((round - prev_round)); fi
  if [[ -n "${prev_blocks}" && "${blocks_proposed:-}" =~ ^[0-9]+$ && "${prev_blocks}" =~ ^[0-9]+$ ]]; then d_blocks=$((blocks_proposed - prev_blocks)); fi
  if [[ -n "${prev_verified}" && "${blocks_verified:-}" =~ ^[0-9]+$ && "${prev_verified}" =~ ^[0-9]+$ ]]; then d_verified=$((blocks_verified - prev_verified)); fi

  prev_round="${round:-$prev_round}"
  prev_blocks="${blocks_proposed:-$prev_blocks}"
  prev_verified="${blocks_verified:-$prev_verified}"

  echo "[NET] peers=$peer_count round=${round:-?} (Δ$d_round) blocks_proposed=${blocks_proposed:-?} (Δ$d_blocks) blocks_verified=${blocks_verified:-?} (Δ$d_verified) rounds_active=${rounds_active:-?} dataset_export=$de_enabled age_s=$de_age last_ts=$de_ts"

  # Account nonce/balance deltas (if hex available)
  if [[ -n "$BOT_HEX" ]]; then
    accb="$(get_account "188.245.97.41" "$BOT_HEX")"
    bot_nonce="$(printf '%s' "$accb" | py_get 'nonce' '?')"
    d_nonce="?"
    if [[ -n "${prev_bot_nonce}" && "${bot_nonce:-}" =~ ^[0-9]+$ && "${prev_bot_nonce}" =~ ^[0-9]+$ ]]; then d_nonce=$((bot_nonce - prev_bot_nonce)); fi
    prev_bot_nonce="${bot_nonce:-$prev_bot_nonce}"
  echo "[ACCT] bot_nonce=${bot_nonce:-?} (Δ$d_nonce)"
  else
    echo "[ACCT] bot_nonce=? (set BOT_HEX=... if your address format differs)"
  fi

  if [[ -n "$USER_HEX" ]]; then
    accu="$(get_account "188.245.97.41" "$USER_HEX")"
    user_bal="$(printf '%s' "$accu" | py_get 'balance_atomic' '?')"
    d_bal="?"
    if [[ -n "${prev_user_bal}" && "${user_bal:-}" =~ ^[0-9]+$ && "${prev_user_bal}" =~ ^[0-9]+$ ]]; then d_bal=$((user_bal - prev_user_bal)); fi
    prev_user_bal="${user_bal:-$prev_user_bal}"
    echo "[ACCT] user_balance_atomic=${user_bal:-?} (Δ$d_bal)"
  fi

  # Per-validator model hash + dataset summary
  for host in "${RPC_HOSTS[@]}"; do
    ai="$(get_ai_status "$host" 2>/dev/null || echo '{}')"
    mh="$(printf '%s' "$ai" | py_get 'model_hash' '?')"
    sm="$(dataset_summary_line "$host")"
    echo "[VAL $host] model_hash=${mh:0:12}... $sm"
  done

  echo
  sleep "$SLEEP_SEC"
done


