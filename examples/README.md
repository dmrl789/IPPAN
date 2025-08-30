# IPPAN Node Connection Examples

This directory contains examples demonstrating how to create and connect IPPAN nodes.

## Examples

### 1. Two Nodes Connect (`two_nodes_connect.rs`)

A simplified demonstration that shows the concept of two nodes connecting and communicating with each other.

**Features:**
- Creates two simple nodes with unique IDs
- Establishes bidirectional connection between nodes
- Demonstrates various message types (Hello, Transaction, Block, Ping/Pong)
- Shows both direct messaging and broadcasting capabilities
- Includes peer discovery simulation

**Run with:**
```bash
cargo run --example two_nodes_connect
```

### 2. Real Nodes Connect (`real_nodes_connect.rs`)

Uses the actual IPPAN node implementation to create and run two real blockchain nodes.

**Features:**
- Creates two full IPPAN nodes with HTTP and P2P ports
- Each node runs with 4 shards for parallel processing
- Includes HTTP API endpoints for health checks and transactions
- Shows how production nodes would be configured
- Demonstrates the full node startup process

**Run with:**
```bash
cargo run --example real_nodes_connect
```

**Note:** This example runs continuously. Press Ctrl+C to stop the nodes.

## Quick Start Scripts

For convenience, we've provided scripts to run these demos:

### Linux/macOS:
```bash
./run_two_nodes_demo.sh
```

### Windows:
```powershell
.\run_two_nodes_demo.ps1
```

## Architecture Overview

### Simple Node Connection
The simplified example demonstrates the core concepts:
1. **Node Creation**: Each node has a unique ID and port configuration
2. **Connection Establishment**: Nodes exchange connection information
3. **Message Exchange**: Various message types are exchanged
4. **Peer Management**: Nodes track connected peers

### Real IPPAN Nodes
The real implementation includes:
1. **Full Node Stack**: Complete blockchain node with all components
2. **HTTP API**: RESTful endpoints for node interaction
3. **P2P Networking**: Actual network layer (currently simplified)
4. **Consensus**: Byzantine Fault Tolerant consensus mechanisms
5. **State Management**: Transaction processing and state updates

## Network Communication

In the current implementation:
- **Simplified Mode**: Direct message passing between nodes (for demonstration)
- **Production Mode**: Would use libp2p for actual P2P communication

### Message Types
- **Hello**: Initial handshake between nodes
- **Transaction**: Financial transactions to be processed
- **Block**: Blocks containing validated transactions
- **Ping/Pong**: Health check and connection maintenance
- **Round Data**: Consensus round information

## Configuration

### Node Configuration
Each node requires:
- **HTTP Port**: For API access (e.g., 8080)
- **P2P Port**: For node-to-node communication (e.g., 9001)
- **Shard Count**: Number of parallel processing shards

### Example Configuration:
```rust
// Node 1
let node1 = Node::new(
    8080,  // HTTP port
    9001,  // P2P port
    4,     // Number of shards
).await?;

// Node 2
let node2 = Node::new(
    8081,  // HTTP port
    9002,  // P2P port
    4,     // Number of shards
).await?;
```

## API Endpoints

When running real nodes, the following endpoints are available:

- `GET /health` - Check node health and status
- `POST /submit_transaction` - Submit a new transaction
- `GET /mempool` - View pending transactions
- `GET /metrics` - Get node performance metrics

## Future Enhancements

The current implementation is simplified. Future enhancements will include:

1. **Full libp2p Integration**: Complete P2P networking with:
   - mDNS for local peer discovery
   - Kademlia DHT for global peer discovery
   - GossipSub for message propagation

2. **Enhanced Security**:
   - Cryptographic signatures on all messages
   - TLS/Noise protocol for encrypted connections
   - Byzantine fault tolerance

3. **Advanced Features**:
   - Automatic peer discovery and connection
   - Dynamic shard rebalancing
   - Cross-shard communication
   - Full consensus implementation

## Development Tips

1. **Logging**: Set `RUST_LOG=info` for detailed logs
2. **Ports**: Ensure ports are not in use before running
3. **Performance**: Use release mode for performance testing: `cargo run --release --example <name>`

## Troubleshooting

### Common Issues:

1. **Port Already in Use**: Change the port numbers in the example
2. **Connection Failed**: Ensure firewall allows the configured ports
3. **Build Errors**: Ensure all dependencies are installed (`libssl-dev`, `pkg-config`)

## Contributing

To add new examples:
1. Create a new `.rs` file in the `examples/` directory
2. Add necessary dependencies to `Cargo.toml`
3. Update this README with documentation
4. Test the example thoroughly