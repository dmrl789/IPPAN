# IPPAN Blockchain

This project implements a next-generation BlockDAG blockchain with BFT consensus and a novel IPPAN median network time mechanism ("HashTimer") for robust, fair, and scalable block validation.

## Key Features

- BlockDAG structure for parallel block production and validation
- Median (IPPAN) time for network-wide consensus on block timestamps
- Gossipsub/libp2p peer discovery and communication
- BFT voting and block approval with validator committees
- Modular Rust code for easy extension and integration

## Getting Started

See `src/main.rs` and `src/blockdag.rs` for main logic and DAG implementation.
