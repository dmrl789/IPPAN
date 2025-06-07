##IPPAN — The Internet’s Peer-to-Peer Autonomous Network

# IPPAN Blockchain

> ⚡️ *A lightweight, efficient, and modular blockchain for real-world decentralization, energy-efficiency, and scalability.*

---

## Overview

**IPPAN** is a next-generation blockchain project aiming to deliver a secure, energy-efficient, and scalable distributed ledger. Unlike traditional blockchains, IPPAN leverages a novel architecture that combines:

- **BlockDAG** for parallel block creation and faster confirmation
- **FBA-inspired (Federated Byzantine Agreement) consensus** for high throughput and decentralized trust
- **Binary encoding** (`bincode`) for ultra-compact storage and fast synchronization
- **Simple, human-readable addresses** for broad accessibility

The project is currently in active development and welcomes contributors, feedback, and research collaboration.

---

## Features

- **Fast block creation** (BlockDAG structure)
- **Energy-efficient mining** (“light mining” with randomized selection)
- **Binary persistent storage** for both blocks and blockchain state
- **Mnemonic or Hex private key backup**
- **Pluggable consensus (FBA, round-robin, etc.)**
- **Human-readable wallet addresses** (e.g. `@username.ipn`)
- **Easy validator and transaction management**
- **Extensible: written in idiomatic Rust**

---

## Getting Started

### Prerequisites

- Rust (>=1.70) and Cargo installed ([Install Rust](https://rustup.rs/))
- Linux or WSL recommended for development

### Build & Run

```bash
# Clone the repository
git clone https://github.com/dmrl789/IPPAN.git
cd IPPAN

# Build the project
cargo build

# Run the wallet/blockchain node
cargo run
