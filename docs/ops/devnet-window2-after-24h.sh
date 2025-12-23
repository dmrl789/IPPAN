#!/usr/bin/env bash
set -euo pipefail

# Window2 closeout (stop tx loop -> collect datasets -> train model -> canonical hash -> optional deploy/activate)
#
# Notes:
# - Canonical hash in this repo is produced by `ippan-ai-core --bin compute_model_hash` (BLAKE3 of canonical JSON).
# - Training uses `ai_training/train_ippan_d_gbdt_devnet.py` (devnet entrypoint).
# - Do not print secrets (key contents). This script only references paths.

MODE="run" # run | check

RPC_BASE="${RPC_BASE:-http://188.245.97.41:8080}"
BOT_HOST="${BOT_HOST:-88.198.26.37}"
VALIDATORS=(
  "188.245.97.41"
  "135.181.145.174"
  "5.223.51.238"
  "178.156.219.107"
)

DATA_LOCAL_DIR="${DATA_LOCAL_DIR:-ai_assets/datasets/devnet_window2}"
MODEL_DIR="${MODEL_DIR:-ai_assets/models/devnet_dlc_window2}"
REMOTE_DATA_DIR="${REMOTE_DATA_DIR:-/var/lib/ippan/ai_datasets}"
REMOTE_MODEL_DIR="${REMOTE_MODEL_DIR:-/opt/ippan/ai/models/devnet_dlc_window2}"
DLC_TOML="${DLC_TOML:-/opt/ippan/config/dlc.toml}"

LOGBOOK="${LOGBOOK:-docs/ops/devnet-tx-bot-window2.log}"

# Activation toggle:
# - default: ACTIVATE env var (0/1), but command-line flags override
ACTIVATE="${ACTIVATE:-0}"

SSH_OPTS="${SSH_OPTS:--o BatchMode=yes -o StrictHostKeyChecking=accept-new -o ConnectTimeout=12 -o ServerAliveInterval=2 -o ServerAliveCountMax=3}"

ts() { date -Is; }
log() { echo "$*" | tee -a "$LOGBOOK"; }

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || { echo "Missing required command: $1" >&2; return 1; }
}

need_one_of() {
  for c in "$@"; do
    if command -v "$c" >/dev/null 2>&1; then return 0; fi
  done
  echo "Missing required command (need one of): $*" >&2
  return 1
}

usage() {
  cat <<'USAGE'
Usage:
  docs/ops/devnet-window2-after-24h.sh [--check] [--activate|--no-activate]

Modes:
  --check         Verify prerequisites + dataset presence + pull datasets locally, then exit.
  --activate      Patch dlc.toml + restart nodes (only in run mode).
  --no-activate   Do not patch/restart (default unless ACTIVATE=1).

Env overrides:
  RPC_BASE, BOT_HOST, SSH_OPTS,
  DATA_LOCAL_DIR, MODEL_DIR, REMOTE_DATA_DIR, REMOTE_MODEL_DIR, DLC_TOML, LOGBOOK,
  ACTIVATE=0|1
USAGE
}

for arg in "$@"; do
  case "$arg" in
    --check) MODE="check" ;;
    --activate) ACTIVATE="1" ;;
    --no-activate) ACTIVATE="0" ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown arg: $arg" >&2; usage >&2; exit 2 ;;
  esac
done

preflight_failed=0
preflight() {
  echo "Preflight: checking required tools..."
  need_cmd ssh || preflight_failed=1
  need_cmd scp || preflight_failed=1
  need_cmd rsync || preflight_failed=1
  need_cmd curl || preflight_failed=1
  need_cmd jq || preflight_failed=1
  need_cmd python3 || preflight_failed=1
  need_cmd cargo || preflight_failed=1
  need_one_of gunzip zcat || preflight_failed=1

  # Quick Python import check for training deps (non-interactive hint only).
  if ! python3 -c 'import pandas, numpy, sklearn, lightgbm' >/dev/null 2>&1; then
    echo "Missing Python packages for training (pandas/numpy/scikit-learn/lightgbm)." >&2
    echo "Hint (Linux): pip install \"numpy==1.26.4\" \"pandas==2.2.2\" \"scikit-learn==1.5.2\" \"lightgbm==4.3.0\" \"blake3==0.4.1\"" >&2
    preflight_failed=1
  fi

  if [[ "$preflight_failed" == "1" ]]; then
    echo "Preflight failed. Install missing tools/deps and re-run." >&2
    echo "Hints:" >&2
    echo "  - Ubuntu: sudo apt-get update && sudo apt-get install -y jq rsync curl openssh-client" >&2
    echo "  - WSL: ensure ssh/scp/rsync/curl installed inside WSL, not just Windows" >&2
    exit 1
  fi
  echo "Preflight OK."
}

require_file_nonempty() {
  local p="$1"
  if [[ ! -s "$p" ]]; then
    echo "ERROR: missing/empty file: $p" >&2
    return 1
  fi
}

is_hex64() {
  [[ "$1" =~ ^[0-9a-fA-F]{64}$ ]]
}

mkdir -p "$(dirname "$LOGBOOK")"
preflight

WINDOW_END="$(ts)"

WINDOW_START="$(grep -E '^WINDOW_START=' "$LOGBOOK" 2>/dev/null | tail -n 1 | cut -d= -f2- || true)"
WINDOW_START="${WINDOW_START:-unknown}"

log "=== Window2 closeout started: $(ts) ==="
log "MODE=$MODE"
log "RPC_BASE=$RPC_BASE"
log "BOT_HOST=$BOT_HOST"
log "ACTIVATE=$ACTIVATE"
log "WINDOW_START=$WINDOW_START"
log "WINDOW_END=$WINDOW_END"

log ""
if [[ "$MODE" == "check" ]]; then
  log "== A) Stop bot tx loop =="
  log "CHECK mode: skipping stop (no remote changes)."
else
  log "== A) Stop bot tx loop =="
  ssh $SSH_OPTS root@"$BOT_HOST" 'ippan-tx-loop-stop && ippan-tx-loop-status' | tee -a "$LOGBOOK" || true
fi

log ""
log "== B) Verify datasets on validators =="
for host in "${VALIDATORS[@]}"; do
  log "--- $host ---"
  ssh $SSH_OPTS root@"$host" "
    set -euo pipefail
    ls -l /etc/ippan/markers/dataset_export_enabled 2>/dev/null || echo 'marker missing'
    ls -lh '$REMOTE_DATA_DIR' 2>/dev/null || echo 'no dataset dir'
    ls -lh '$REMOTE_DATA_DIR'/devnet_dataset_*.csv.gz 2>/dev/null | tail -n 5 || echo 'no devnet_dataset_*.csv.gz'
    du -sh '$REMOTE_DATA_DIR' 2>/dev/null || true
  " | tee -a "$LOGBOOK"
done

log ""
log "== C) Pull datasets locally =="
mkdir -p "$DATA_LOCAL_DIR"

# Per-host staging avoids overwrites and makes it obvious who produced what.
for host in "${VALIDATORS[@]}"; do
  outdir="$DATA_LOCAL_DIR/$host"
  mkdir -p "$outdir"
  log "--- pull from $host -> $outdir ---"

  # Prefer rsync; fallback to scp if remote lacks rsync.
  if rsync -avz -e "ssh $SSH_OPTS" root@"$host":"$REMOTE_DATA_DIR"/devnet_dataset_*.csv.gz "$outdir"/ >/dev/null 2>&1; then
    log "rsync ok"
  else
    log "rsync failed (remote may not have rsync). Falling back to scp."
    files="$(ssh $SSH_OPTS root@"$host" "ls -1 '$REMOTE_DATA_DIR'/devnet_dataset_*.csv.gz 2>/dev/null || true")"
    if [[ -z "$files" ]]; then
      log "no files from $host"
      continue
    fi
    while IFS= read -r f; do
      [[ -n "$f" ]] || continue
      scp $SSH_OPTS root@"$host":"$f" "$outdir"/ >/dev/null
      log "fetched $(basename "$f")"
    done <<<"$files"
  fi
done

log ""
log "Local dataset files:"
find "$DATA_LOCAL_DIR" -maxdepth 2 -name 'devnet_dataset_*.csv.gz' -type f -print | wc -l | awk '{print "DATASET_FILES_LOCAL_TOTAL=" $1}' | tee -a "$LOGBOOK"
for host in "${VALIDATORS[@]}"; do
  cnt="$(find "$DATA_LOCAL_DIR/$host" -maxdepth 1 -name 'devnet_dataset_*.csv.gz' -type f 2>/dev/null | wc -l | tr -d ' ')"
  log "DATASET_FILES_LOCAL_$host=$cnt"
done
ls -lh "$DATA_LOCAL_DIR" | tee -a "$LOGBOOK" || true

log ""
log "== D) Merge datasets =="
FLAT="$DATA_LOCAL_DIR/_flat"
rm -rf "$FLAT"
mkdir -p "$FLAT"
find "$DATA_LOCAL_DIR" -name 'devnet_dataset_*.csv.gz' -type f -print0 | xargs -0 -I{} cp -f {} "$FLAT"/

DATASET_COUNT="$(ls -1 "$FLAT"/devnet_dataset_*.csv.gz 2>/dev/null | wc -l | tr -d ' ')"
log "DATASET_FILES_FLAT=$DATASET_COUNT"
if [[ "$DATASET_COUNT" == "0" ]]; then
  log "ERROR: no dataset files found locally after pull; aborting."
  exit 1
fi

MERGED="$DATA_LOCAL_DIR/merged_devnet_window2.csv"
if [[ -f "ai_training/merge_devnet_datasets.py" ]]; then
  python3 ai_training/merge_devnet_datasets.py --input-dir "$FLAT" --output "$MERGED" --dedupe | tee -a "$LOGBOOK"
else
  log "ERROR: ai_training/merge_devnet_datasets.py not found"
  exit 1
fi
ls -lh "$MERGED" | tee -a "$LOGBOOK"

if [[ "$MODE" == "check" ]]; then
  log ""
  log "Check mode complete (no training, no deploy)."
  exit 0
fi

log ""
log "== E) Train window2 model =="
mkdir -p "$MODEL_DIR"

if [[ -d ".venv-dlc-train" ]]; then
  # shellcheck disable=SC1091
  source .venv-dlc-train/bin/activate
fi

if [[ ! -f "ai_training/train_ippan_d_gbdt_devnet.py" ]]; then
  log "ERROR: ai_training/train_ippan_d_gbdt_devnet.py not found"
  exit 1
fi

# Train and tee output into training.log
python3 ai_training/train_ippan_d_gbdt_devnet.py \
  --csv "$MERGED" \
  --out "$MODEL_DIR/model.json" \
  --model-id "devnet_dlc_window2" \
  2>&1 | tee "$MODEL_DIR/training.log" | tee -a "$LOGBOOK"

require_file_nonempty "$MODEL_DIR/model.json"
require_file_nonempty "$MODEL_DIR/training.log"

# Validate JSON (structural sanity only; avoids empty/partial writes).
jq -e . "$MODEL_DIR/model.json" >/dev/null

log "Training outputs:"
ls -lh "$MODEL_DIR" | tee -a "$LOGBOOK"

log ""
log "== F) Canonical model hash (Rust) =="
cargo build --release -p ippan-ai-core --bin compute_model_hash --locked
./target/release/compute_model_hash "$MODEL_DIR/model.json" | tee "$MODEL_DIR/model.hash" | tee -a "$LOGBOOK"

require_file_nonempty "$MODEL_DIR/model.hash"
MODEL_HASH="$(tr -d '\r\n' < "$MODEL_DIR/model.hash")"
if ! is_hex64 "$MODEL_HASH"; then
  log "ERROR: model hash is not 64 hex chars: $MODEL_HASH"
  exit 1
fi
log "MODEL_HASH_WINDOW2=$MODEL_HASH"

log ""
log "== G) Upload model artifacts to validators =="
for host in "${VALIDATORS[@]}"; do
  log "--- $host ---"
  ssh $SSH_OPTS root@"$host" "mkdir -p '$REMOTE_MODEL_DIR'"
  scp $SSH_OPTS "$MODEL_DIR/model.json" "$MODEL_DIR/model.hash" "$MODEL_DIR/training.log" root@"$host":"$REMOTE_MODEL_DIR"/ >/dev/null
done

if [[ "$ACTIVATE" != "1" ]]; then
  log ""
  log "ACTIVATE=0: skipping dlc.toml patch + restarts."
else
  log ""
  log "== H) Activate window2 model (patch dlc.toml + restart) =="
  NEW_MODEL_PATH="$REMOTE_MODEL_DIR/model.json"
  NEW_MODEL_HASH="$MODEL_HASH"

  for host in "${VALIDATORS[@]}"; do
    log "--- $host ---"
    ssh $SSH_OPTS root@"$host" "
      set -euo pipefail
      bak='${DLC_TOML}.bak.window2.'\$(date +%Y%m%d%H%M%S)
      cp '$DLC_TOML' \"\$bak\"
      perl -0777 -pi -e '
        s|(\\[dgbdt\\.model\\][^\\[]*?\\n\\s*path\\s*=\\s*)\"[^\"]+\"|\\1\"$NEW_MODEL_PATH\"|s;
        s|(\\[dgbdt\\.model\\][^\\[]*?\\n\\s*expected_hash\\s*=\\s*)\"[^\"]+\"|\\1\"$NEW_MODEL_HASH\"|s;
      ' '$DLC_TOML'

      echo \"Backup: \$bak\"
      echo \"Post-patch [dgbdt.model] section:\"
      awk '
        BEGIN{in=0}
        /^\\[dgbdt\\.model\\]/{in=1}
        /^\\[/{if(in && \$0 !~ /^\\[dgbdt\\.model\\]/){exit}}
        {if(in) print}
      ' '$DLC_TOML'

      systemctl restart ippan-node
      sleep 3
      systemctl is-active ippan-node
    " | tee -a "$LOGBOOK"
  done
fi

log ""
log "== I) Verify /ai/status on all validators =="
for host in "${VALIDATORS[@]}"; do
  log "--- $host ---"
  curl -fsS "http://$host:8080/ai/status" | tee -a "$LOGBOOK"
  echo | tee -a "$LOGBOOK"
done

log "=== Window2 closeout finished: $(ts) ==="


