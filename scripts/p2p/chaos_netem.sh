#!/bin/bash
set -e

echo "Building ippan-node with testkit features..."
cargo build -p ippan-node --features p2p-testkit

echo "Building p2p-testkit..."
cargo build -p p2p-testkit

echo "Running CHAOS test..."
export IPPAN_NODE_CMD="./target/debug/ippan-node"
cargo run -p p2p-testkit -- chaos --nodes 6 --loss 2 --latency-ms 80 --jitter-ms 20 --msgs 200 --size 256 --min-delivery-rate 0.70
