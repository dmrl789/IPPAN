#!/usr/bin/env bash
# ramp_batch_200k_v2.sh — 200k-500k offered TPS batch ingest benchmark
#
# GOAL: client_errors=0, invalid=0, overload manifests as HTTP 429 only.
#
# Usage (on api1):
#   cd /var/lib/ippan
#   RPC=http://127.0.0.1:8080 \
#   SENDERS_FILE=out/senders/senders_1024.json \
#   TO_ADDR=<NOT-A-SENDER-ADDRESS> \
#   bash scripts/ops/ramp_batch_200k_v2.sh
#
# Requirements:
#   - ippan-txload binary in /usr/local/bin
#   - Funded senders file (>99% funded)
#   - TO_ADDR must NOT be in the senders file

set -euo pipefail

# ============================================================================
# Configuration (tuned for client_errors=0)
# ============================================================================
RPC="${RPC:-http://127.0.0.1:8080}"
SENDERS_FILE="${SENDERS_FILE:-out/senders/senders_1024.json}"
TO_ADDR="${TO_ADDR:-}"
TXLOAD="${TXLOAD:-/usr/local/bin/ippan-txload}"
OUT_DIR="${OUT_DIR:-out/batch_$(date -u +%Y%m%dT%H%M%SZ)}"

# Tuning: conservative settings to avoid socket exhaustion / timeouts
BATCH_SIZE=512           # Smaller batches = less per-request latency
CONCURRENCY=128          # Not too many parallel connections
MAX_INFLIGHT=65536       # Large enough for buffering
MAX_QUEUE=500000         # Large client queue
DRAIN_SECONDS=20         # Allow drain after test

# ============================================================================
# Pre-flight checks
# ============================================================================
if [[ -z "$TO_ADDR" ]]; then
    echo "ERROR: TO_ADDR must be set (must not be in senders file)"
    exit 1
fi

if [[ ! -f "$SENDERS_FILE" ]]; then
    echo "ERROR: SENDERS_FILE not found: $SENDERS_FILE"
    exit 1
fi

# Check TO_ADDR not in senders file
if grep -q "\"address\": \"$TO_ADDR\"" "$SENDERS_FILE"; then
    echo "ERROR: TO_ADDR ($TO_ADDR) is in senders file — from==to will fail"
    echo "Use a different TO_ADDR that is NOT a sender."
    exit 1
fi

if ! command -v "$TXLOAD" &>/dev/null; then
    echo "ERROR: ippan-txload not found at $TXLOAD"
    exit 1
fi

# Raise file descriptor limit
ulimit -n 200000 2>/dev/null || true

# Create output directory
mkdir -p "$OUT_DIR"

echo "============================================"
echo "Batch Ingest Benchmark (WireV1 format)"
echo "============================================"
echo "RPC:          $RPC"
echo "SENDERS:      $SENDERS_FILE"
echo "TO_ADDR:      $TO_ADDR"
echo "OUT_DIR:      $OUT_DIR"
echo "BATCH_SIZE:   $BATCH_SIZE"
echo "CONCURRENCY:  $CONCURRENCY"
echo "MAX_INFLIGHT: $MAX_INFLIGHT"
echo ""

# ============================================================================
# Health check
# ============================================================================
echo "Checking node health..."
curl -sS -m 5 "$RPC/health" || { echo "ERROR: Node not healthy"; exit 1; }
echo ""

# ============================================================================
# Sanity test (2k TPS, 10s)
# ============================================================================
echo ""
echo "=== Stage 0: Sanity test (2k offered, 10s) ==="
NONCE_START=1
"$TXLOAD" batch \
    --rpc "$RPC" \
    --tps 2000 \
    --seconds 10 \
    --senders-file "$SENDERS_FILE" \
    --to "$TO_ADDR" \
    --nonce-start "$NONCE_START" \
    --batch-size 256 \
    --concurrency 32 \
    --max-inflight 2048 \
    --max-queue 100000 \
    --drain-seconds 10 \
    | tee "$OUT_DIR/sanity_2k.log"

# Check sanity pass
if grep -q "invalid=0" "$OUT_DIR/sanity_2k.log" && grep -q "client_errors=0" "$OUT_DIR/sanity_2k.log"; then
    echo "✓ Sanity passed: invalid=0, client_errors=0"
else
    echo "✗ Sanity FAILED — check $OUT_DIR/sanity_2k.log"
    exit 1
fi

# ============================================================================
# Stage 1: 200k offered (20s)
# ============================================================================
echo ""
echo "=== Stage 1: 200k offered (20s) ==="
NONCE_START=50
"$TXLOAD" batch \
    --rpc "$RPC" \
    --tps 200000 \
    --seconds 20 \
    --senders-file "$SENDERS_FILE" \
    --to "$TO_ADDR" \
    --nonce-start "$NONCE_START" \
    --batch-size "$BATCH_SIZE" \
    --concurrency "$CONCURRENCY" \
    --max-inflight "$MAX_INFLIGHT" \
    --max-queue "$MAX_QUEUE" \
    --drain-seconds "$DRAIN_SECONDS" \
    | tee "$OUT_DIR/batch_200k_20s.log"

# ============================================================================
# Stage 2: 300k offered (20s)
# ============================================================================
echo ""
echo "=== Stage 2: 300k offered (20s) ==="
NONCE_START=500
"$TXLOAD" batch \
    --rpc "$RPC" \
    --tps 300000 \
    --seconds 20 \
    --senders-file "$SENDERS_FILE" \
    --to "$TO_ADDR" \
    --nonce-start "$NONCE_START" \
    --batch-size "$BATCH_SIZE" \
    --concurrency "$CONCURRENCY" \
    --max-inflight "$MAX_INFLIGHT" \
    --max-queue "$MAX_QUEUE" \
    --drain-seconds "$DRAIN_SECONDS" \
    | tee "$OUT_DIR/batch_300k_20s.log"

# ============================================================================
# Stage 3: 500k offered (20s)
# ============================================================================
echo ""
echo "=== Stage 3: 500k offered (20s) ==="
NONCE_START=1000
"$TXLOAD" batch \
    --rpc "$RPC" \
    --tps 500000 \
    --seconds 20 \
    --senders-file "$SENDERS_FILE" \
    --to "$TO_ADDR" \
    --nonce-start "$NONCE_START" \
    --batch-size "$BATCH_SIZE" \
    --concurrency "$CONCURRENCY" \
    --max-inflight "$MAX_INFLIGHT" \
    --max-queue "$MAX_QUEUE" \
    --drain-seconds "$DRAIN_SECONDS" \
    | tee "$OUT_DIR/batch_500k_20s.log"

# ============================================================================
# Stage 4: 500k offered (60s) — only if previous stages are clean
# ============================================================================
if grep -q "invalid=0" "$OUT_DIR/batch_500k_20s.log" && grep -q "client_errors=0" "$OUT_DIR/batch_500k_20s.log"; then
    echo ""
    echo "=== Stage 4: 500k offered (60s) ==="
    NONCE_START=2000
    "$TXLOAD" batch \
        --rpc "$RPC" \
        --tps 500000 \
        --seconds 60 \
        --senders-file "$SENDERS_FILE" \
        --to "$TO_ADDR" \
        --nonce-start "$NONCE_START" \
        --batch-size "$BATCH_SIZE" \
        --concurrency "$CONCURRENCY" \
        --max-inflight "$MAX_INFLIGHT" \
        --max-queue 1000000 \
        --drain-seconds 30 \
        | tee "$OUT_DIR/batch_500k_60s.log"
else
    echo ""
    echo "⚠ Skipping 60s stage — previous stage had client_errors or invalid > 0"
fi

# ============================================================================
# Summary
# ============================================================================
echo ""
echo "============================================"
echo "SUMMARY"
echo "============================================"
echo ""
for f in "$OUT_DIR"/*.log; do
    echo "--- $(basename "$f") ---"
    grep "SUMMARY" "$f" || echo "(no SUMMARY line)"
done

echo ""
echo "Final health check:"
curl -sS -m 5 "$RPC/health"
echo ""

# Check if all stages achieved client_errors=0
ALL_CLEAN=true
for f in "$OUT_DIR"/*.log; do
    if ! grep -q "client_errors=0" "$f"; then
        ALL_CLEAN=false
    fi
    if ! grep -q "invalid=0" "$f"; then
        ALL_CLEAN=false
    fi
done

if $ALL_CLEAN; then
    echo ""
    echo "✓ ALL STAGES PASSED: invalid=0, client_errors=0"
    echo "  Overload correctly manifests as http_429."
else
    echo ""
    echo "⚠ Some stages had client_errors > 0 or invalid > 0"
    echo "  Consider reducing CONCURRENCY or increasing server capacity."
fi

echo ""
echo "Output: $OUT_DIR"

