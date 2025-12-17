#!/usr/bin/env bash
set -euo pipefail

LOCK="/var/lock/ippan-export-dataset.lock"

OUT_DIR="/var/lib/ippan/ai_datasets"
TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_CSV="$OUT_DIR/devnet_dataset_${TS}.csv"

REPO_DIR="/root/IPPAN"
RPC_URL="http://127.0.0.1:8080"

# low-noise sampling (tweak if desired)
SAMPLES="120"
INTERVAL="5"

# retention caps
MAX_FILES="200"
MAX_DIR_MB="2048"

export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

mkdir -p "$OUT_DIR"

# avoid overlapping runs
exec 9>"$LOCK"
if ! flock -n 9; then
  echo "Another export run is already in progress; exiting."
  exit 0
fi

command -v python3 >/dev/null
python3 -c "import requests" >/dev/null

if [ ! -d "$REPO_DIR" ]; then
  echo "ERROR: repo dir not found: $REPO_DIR" >&2
  exit 2
fi

if [ ! -f "$REPO_DIR/ai_training/export_localnet_dataset.py" ]; then
  echo "ERROR: exporter not found: $REPO_DIR/ai_training/export_localnet_dataset.py" >&2
  exit 2
fi

cd "$REPO_DIR"

python3 ai_training/export_localnet_dataset.py \
  --mode rpc \
  --rpc "$RPC_URL" \
  --samples "$SAMPLES" \
  --interval "$INTERVAL" \
  --out "$OUT_CSV"

gzip -f "$OUT_CSV"

# verify output exists + non-empty
test -s "${OUT_CSV}.gz"

# retention by count
ls -1t "$OUT_DIR"/devnet_dataset_*.csv.gz 2>/dev/null | tail -n +$((MAX_FILES+1)) | xargs -r rm -f

# retention by total size
while [ "$(du -sm "$OUT_DIR" | awk '{print $1}')" -gt "$MAX_DIR_MB" ]; do
  oldest="$(ls -1t "$OUT_DIR"/devnet_dataset_*.csv.gz 2>/dev/null | tail -n 1)"
  [ -n "$oldest" ] || break
  rm -f "$oldest"
done


