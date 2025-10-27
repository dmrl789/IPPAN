# IPPAN RPC Server

A high-performance RPC server for the IPPAN blockchain network, built with Rust and Axum.

## Features

- **HTTP/HTTPS RPC API** - RESTful API for blockchain operations
- **P2P Network Integration** - Built-in peer-to-peer networking
- **Web UI** - Modern web interface for monitoring and interaction
- **Metrics Endpoint** - Prometheus-compatible metrics
- **CORS Support** - Cross-origin resource sharing enabled
- **Async/Await** - Built on Tokio for high concurrency

## API Endpoints

### Health & Info
- `GET /health` - Health check and node status
- `GET /time` - Current timestamp
- `GET /version` - Version information
- `GET /metrics` - Prometheus metrics

### Blockchain Operations
- `POST /transactions` - Submit a transaction
- `GET /blocks` - Get block information
- `GET /accounts/:address` - Get account information

### P2P Network
- `GET /p2p/peers` - List connected peers
- `POST /p2p/peers` - Add a new peer
- `DELETE /p2p/peers/:address` - Remove a peer
- `POST /p2p/blocks` - P2P block messages
- `POST /p2p/transactions` - P2P transaction messages
- `POST /p2p/peer-info` - P2P peer information

### Web Interface
- `GET /` - Web UI dashboard
- `GET /static/*` - Static assets

## Usage

### Basic Server

```rust
use ippan_rpc::{start_server, L2Config, P2PConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Configure P2P network
    let p2p_config = P2PConfig {
        listen_address: "http://0.0.0.0:9000".to_string(),
        bootstrap_peers: vec![],
        max_peers: 50,
        peer_discovery_interval: Duration::from_secs(30),
        message_timeout: Duration::from_secs(10),
        retry_attempts: 3,
        public_host: None,
        enable_upnp: false,
        external_ip_services: vec![
            "https://api.ipify.org".to_string(),
            "https://ifconfig.me/ip".to_string(),
        ],
        peer_announce_interval: Duration::from_secs(60),
    };

    // Configure L2 blockchain
    let l2_config = L2Config::default();

    // Start the RPC server
    start_server("http://0.0.0.0:8080", p2p_config, l2_config).await?;

    Ok(())
}
```

### Running the Example

```bash
cargo run --example simple_server
```

The server will start on:
- RPC API: http://localhost:8080
- P2P Network: http://localhost:9000
- Web UI: http://localhost:8080

## Configuration

### P2P Configuration

```rust
let p2p_config = P2PConfig {
    listen_address: "http://0.0.0.0:9000".to_string(),
    bootstrap_peers: vec!["http://peer1.example.com:9000".to_string()],
    max_peers: 100,
    peer_discovery_interval: Duration::from_secs(30),
    message_timeout: Duration::from_secs(10),
    retry_attempts: 3,
    public_host: Some("https://my-node.example.com".to_string()),
    enable_upnp: true,
    external_ip_services: vec![
        "https://api.ipify.org".to_string(),
        "https://ifconfig.me/ip".to_string(),
    ],
    peer_announce_interval: Duration::from_secs(60),
};
```

### L2 Configuration

The `L2Config` struct contains extensive configuration options for the blockchain layer, including:

- Chain ID and block timing
- Transaction limits and gas settings
- Consensus parameters
- Contract execution limits
- Rate limiting settings

## Dependencies

- **axum** - Web framework
- **tokio** - Async runtime
- **serde** - Serialization
- **reqwest** - HTTP client
- **igd** - UPnP support
- **blake3** - Hashing
- **bincode** - Binary serialization

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Running Tests

```bash
cargo test --package ippan-rpc
```

## Architecture

The RPC server is built with a modular architecture:

- **lib.rs** - Core P2P network implementation
- **server.rs** - HTTP server and API endpoints
- **static/** - Web UI assets

The server integrates with the IPPAN blockchain ecosystem through the `ippan-types` and `ippan-storage` crates.

## License

This project is part of the IPPAN blockchain ecosystem and follows the same licensing terms.