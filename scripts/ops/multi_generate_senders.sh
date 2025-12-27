#!/usr/bin/env bash
set -euo pipefail

# Generate a senders file for multi-sender load tests.
#
# Usage:
#   ./scripts/ops/multi_generate_senders.sh 20

N="${1:-}"
if [[ -z "$N" ]]; then
  echo "Usage: $0 <count>" >&2
  exit 2
fi

OUT="${OUT:-out/senders/senders.json}"
mkdir -p "$(dirname "$OUT")"

exec cargo run --release -p ippan-txload -- gen-senders --count "$N" --out "$OUT"


