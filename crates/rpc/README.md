# IPPAN RPC Server

**Production-ready RPC server for the IPPAN blockchain network**, built with Rust and Axum.

---

## Overview

The `ippan-rpc` crate provides a **high-performance HTTP/JSON-RPC API** for IPPAN nodes.  
It manages blockchain operations such as transactions, blocks, accounts, and P2P communication, and can optionally serve the Unified Web UI for full-stack deployment.

---

## ✨ Features

### ✅ Production-Ready Components

- **HTTP/JSON-RPC API** — Full REST API for blockchain interactions  
- **CORS Support** — Configurable cross-origin resource sharing  
- **Structured Logging & Tracing** — Built-in with `tracing` crate  
- **Comprehensive Error Handling** — Proper HTTP codes & messages  
- **P2P Integration** — HTTP-based networking via `ippan-p2p`  
- **Layer 2 Support** — L2 commit, exit, and network coordination  
- **Static Asset Hosting** — Serve Unified UI directly from RPC node  
- **Metrics Endpoint** — Prometheus-compatible `/metrics` exporter  
- **Async Runtime** — Built on `tokio` for concurrent performance  

---

## 🌐 API Endpoints

### Health & Monitoring
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Node health & consensus status |
| `GET` | `/time` | Current IPPAN and Unix timestamps |
| `GET` | `/version` | Node version and build info |
| `GET` | `/metrics` | Prometheus metrics endpoint |

### Blockchain Operations
| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/tx` | Submit a new transaction |
| `GET` | `/tx/:hash` | Retrieve transaction by hash |
| `GET` | `/block/:id` | Retrieve block by height or hash |
| `GET` | `/account/:address` | Query account information |
| `GET` | `/peers` | List connected peers |

### P2P Networking (Internal)
| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/p2p/blocks` | Receive block from peer |
| `POST` | `/p2p/transactions` | Receive transaction from peer |
| `POST` | `/p2p/block-request` | Request block data |
| `POST` | `/p2p/block-response` | Respond with block data |
| `POST` | `/p2p/peer-info` | Handle peer announcement |
| `POST` | `/p2p/peer-discovery` | Handle discovery requests |
| `GET` | `/p2p/peers` | Return list of active peers |

### Layer 2
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/l2/config` | Retrieve current L2 configuration |
| `GET` | `/l2/networks` | List available L2 networks |
| `GET` | `/l2/commits` | List commits (filterable by `?l2_id=`) |
| `GET` | `/l2/commits/:l2_id` | Commits for specific L2 |
| `GET` | `/l2/exits` | List L2 exit records |
| `GET` | `/l2/exits/:l2_id` | Exits for specific L2 |

---

## ⚙️ Architecture

### Core Components
1. **AppState** — Shared runtime state (storage, mempool, consensus, etc.)  
2. **ConsensusHandle** — Thread-safe wrapper for consensus operations  
3. **Router** — Axum-based HTTP router with middleware layers  
4. **Error Layer** — Unified error responses with status codes  

### Middleware Stack
1. **CORS Layer** — Cross-origin request handling  
2. **Trace Layer** — Structured request/response logging  
3. **State Layer** — Application state injection  

---

## 🚀 Usage Example

```rust
use ippan_rpc::{start_server, AppState, L2Config};
use std::sync::{Arc, atomic::AtomicUsize};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let l2_config = L2Config {
        max_commit_size: 1000,
        min_epoch_gap_ms: 5000,
        challenge_window_ms: 10000,
        da_mode: "optimistic".into(),
        max_l2_count: 100,
    };

    let state = AppState {
        storage: Arc::new(/* storage */),
        start_time: Instant::now(),
        peer_count: Arc::new(AtomicUsize::new(0)),
        p2p_network: None,
        tx_sender: None,
        node_id: "node-1".into(),
        consensus_mode: "poa".into(),
        consensus: None,
        ai_status: None,
        l2_config,
        mempool: Arc::new(/* mempool */),
        unified_ui_dist: None,
        req_count: Arc::new(AtomicUsize::new(0)),
        security: None,
        metrics: None,
        file_storage: None,
        file_dht: None,
        dht_file_mode: "stub".into(),
        dev_mode: false,
        handle_registry: Arc::new(/* registry */),
        handle_anchors: Arc::new(/* anchors */),
        handle_dht: None,
        dht_handle_mode: "stub".into(),
    };

    start_server(state, "0.0.0.0:9000").await?;
    Ok(())
}
