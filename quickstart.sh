#!/bin/bash

echo "🚀 IPPAN Neural Blockchain Quickstart"
echo "====================================="

# Create data directories
echo "📁 Creating data directories..."
mkdir -p data/{models,datasets,jobs,proofs}

# Build all components
echo "🔨 Building neural blockchain components..."
cargo build --release

# Start the API server in background
echo "🌐 Starting Neuro API server..."
cargo run --release -p neuro-api &
API_PID=$!

# Wait for API to start
sleep 3

# Test health check
echo "🏥 Testing health check..."
curl -s http://localhost:3000/health || echo "API not ready yet, waiting..."
sleep 2

# Create a test model
echo "🤖 Creating test model..."
TEST_OWNER="$(openssl rand -hex 32)"
TEST_WEIGHTS="$(openssl rand -hex 32)"

cargo run --release -p neuro-cli -- create-model \
    --owner "$TEST_OWNER" \
    --arch-id 1 \
    --version 1 \
    --weights-hash "$TEST_WEIGHTS" \
    --size-bytes 1000000 \
    --license-id 1

# Create a test job
echo "📋 Creating test inference job..."
TEST_MODEL_REF="$TEST_WEIGHTS"
TEST_INPUT="$(openssl rand -hex 32)"

cargo run --release -p neuro-cli -- create-job \
    --model-ref "$TEST_MODEL_REF" \
    --input-commit "$TEST_INPUT" \
    --max-latency-ms 1000 \
    --region "us-east-1" \
    --max-price-ipn 1000000 \
    --escrow-ipn 500000 \
    --privacy "open" \
    --bid-window-ms 30000

# Place a test bid
echo "💰 Placing test bid..."
TEST_EXECUTOR="$(openssl rand -hex 32)"
JOB_ID="$(cargo run --release -p neuro-cli -- create-job \
    --model-ref "$TEST_MODEL_REF" \
    --input-commit "$TEST_INPUT" \
    --max-latency-ms 1000 \
    --region "us-east-1" \
    --max-price-ipn 1000000 \
    --escrow-ipn 500000 \
    --privacy "open" \
    --bid-window-ms 30000 | jq -r '.id')"

cargo run --release -p neuro-cli -- place-bid \
    --job-id "$JOB_ID" \
    --executor-id "$TEST_EXECUTOR" \
    --price-ipn 800000 \
    --est-latency-ms 800 \
    --tee false

# Start executor in background
echo "⚙️  Starting Neuro Executor..."
cargo run --release -p neuro-executor &
EXECUTOR_PID=$!

echo ""
echo "✅ Neural Blockchain components are running!"
echo "   - API Server: http://localhost:3000"
echo "   - CLI Tool: cargo run --release -p neuro-cli -- --help"
echo "   - Executor: Running in background"
echo ""
echo "Press Ctrl+C to stop all components"

# Wait for interrupt
trap "echo '🛑 Stopping components...'; kill $API_PID $EXECUTOR_PID 2>/dev/null; exit" INT
wait
