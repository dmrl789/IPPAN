#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="${COMPOSE_FILE:-$ROOT_DIR/localnet/docker-compose.full-stack.yaml}"
PROJECT_NAME="${PROJECT_NAME:-ippan-local}"

if [[ ! -f "$COMPOSE_FILE" ]]; then
  echo "error: compose file not found at $COMPOSE_FILE" >&2
  exit 1
fi

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: '$1' is required but not installed" >&2
    exit 1
  fi
}

require_cmd docker

if docker compose version >/dev/null 2>&1; then
  COMPOSE_BIN=(docker compose)
elif command -v docker-compose >/dev/null 2>&1; then
  COMPOSE_BIN=(docker-compose)
else
  echo "error: docker compose plugin or docker-compose binary is required" >&2
  exit 1
fi

DATA_DIR="$ROOT_DIR/localnet/data/node"
mkdir -p "$DATA_DIR"

echo "▶️  Building and starting the IPPAN local full-stack environment..."
"${COMPOSE_BIN[@]}" -f "$COMPOSE_FILE" -p "$PROJECT_NAME" up --build -d

echo
"${COMPOSE_BIN[@]}" -f "$COMPOSE_FILE" -p "$PROJECT_NAME" ps

cat <<'INSTRUCTIONS'

✅ Services are starting in the background.

Visit the following once the containers report "healthy":
  • Node RPC:      http://localhost:8080/health
  • Gateway API:   http://localhost:8081/api/health
  • Unified UI:    http://localhost:3000

Next steps:
  1. Generate a devnet wallet:
       ippan-wallet --network devnet generate-key --out ./keys/dev.key --prompt-password
  2. Fund it via /dev/fund (dev mode) or use the faucet.
  3. Send a payment:
       ippan-wallet --rpc-url http://localhost:8081 send-payment --key ./keys/dev.key --prompt-password --to @friend.ipn --amount 0.1 --memo "hello"
  4. Watch the transaction appear in the explorer UI.

To follow logs:
  ${COMPOSE_BIN[@]} -f localnet/docker-compose.full-stack.yaml -p ippan-local logs -f

To stop everything:
  ${COMPOSE_BIN[@]} -f localnet/docker-compose.full-stack.yaml -p ippan-local down

INSTRUCTIONS
