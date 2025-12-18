#!/usr/bin/env bash
set -euo pipefail

DATA_DIR="${1:-ai_assets/datasets/devnet}"
OUT_DIR="${2:-ai_assets/models/devnet}"

mkdir -p "$DATA_DIR" "$OUT_DIR"

python3 ai_training/train_ippan_d_gbdt_devnet.py \
  --data-dir "$DATA_DIR" \
  --output-dir "$OUT_DIR"

echo "D-GBDT devnet training finished."


