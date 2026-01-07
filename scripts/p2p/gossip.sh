#!/bin/bash
set -e

echo "Building ippan-node with testkit features..."
cargo build -p ippan-node --features p2p-testkit

echo "Building p2p-testkit..."
cargo build -p p2p-testkit

echo "Running GOSSIP test..."
export IPPAN_NODE_CMD="./target/debug/ippan-node"
cargo run -p p2p-testkit -- gossip --nodes 8 --msgs 200 --size 512
