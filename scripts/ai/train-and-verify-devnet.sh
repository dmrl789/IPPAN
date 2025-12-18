#!/usr/bin/env bash
set -euo pipefail

DATA_DIR="${1:-ai_assets/datasets/devnet}"
OUT_DIR="${2:-ai_assets/models/devnet}"

mkdir -p "$DATA_DIR" "$OUT_DIR"

TMP_LOG="$(mktemp)"
trap 'rm -f "$TMP_LOG"' EXIT

# 1) Train model
python3 ai_training/train_ippan_d_gbdt_devnet.py \
  --data-dir "$DATA_DIR" \
  --output-dir "$OUT_DIR" \
  | tee "$TMP_LOG"

# 2) Extract model path + hash if trainer prints them (key=value)
MODEL_PATH="$(grep -E '^model_path=' "$TMP_LOG" | tail -n 1 | cut -d= -f2- || true)"
MODEL_HASH="$(grep -E '^model_hash=' "$TMP_LOG" | tail -n 1 | cut -d= -f2- || true)"
MODEL_ID="$(grep -E '^model_id=' "$TMP_LOG" | tail -n 1 | cut -d= -f2- || true)"

echo "MODEL_ID=${MODEL_ID}"
echo "MODEL_PATH=${MODEL_PATH}"
echo "MODEL_HASH=${MODEL_HASH}"

# 3) Ready to wire: run the existing hash verifier once config/dlc.toml points to this model:
# cargo run -q -p ippan-ai-core --bin verify_model_hash -- config/dlc.toml

echo "Train+verify pipeline completed (hash verification step is ready to wire)."


