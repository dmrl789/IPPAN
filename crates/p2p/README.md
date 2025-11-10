# IPPAN P2P

## Overview
The p2p crate implements production-ready networking for IPPAN nodes with two implementation options:

1. **libp2p Network (Recommended)** - Full-featured P2P stack with:
   - Kademlia DHT for distributed peer discovery
   - GossipSub for efficient message broadcasting  
   - mDNS for local network peer discovery
   - Relay and DCUtR protocols for NAT traversal
   - Request/Response protocol for direct peer queries
   - Comprehensive error handling and logging

2. **HTTP P2P Network (Legacy)** - Simple HTTP-based networking for testing/development

## Key Features

### libp2p Implementation
- **Kademlia DHT**: Distributed hash table for peer discovery and content routing
- **GossipSub**: Efficient pub/sub messaging for blocks and transactions
- **mDNS**: Zero-config local network peer discovery
- **NAT Traversal**: Relay and DCUtR protocols for connectivity behind NATs/firewalls
- **Identify Protocol**: Automatic peer information exchange
- **Request/Response**: Direct peer-to-peer queries for blocks and peer lists
- **Connection Management**: Automatic connection lifecycle management

### HTTP P2P (Legacy)
- Built-in UPnP support for port mapping
- External IP detection via multiple services
- Peer metadata tracking and health monitoring
- HTTP-based message broadcasting

## Usage

### libp2p Network

```rust
use ippan_p2p::{LibP2PConfig, LibP2PNetwork};

// Create configuration
let config = LibP2PConfig {
    listen_addresses: vec![
        "/ip4/0.0.0.0/tcp/9000".parse().unwrap(),
    ],
    bootstrap_peers: vec![
        // Add bootstrap peers here
    ],
    enable_mdns: true,
    enable_relay: true,
    idle_connection_timeout: Duration::from_secs(60),
};

// Create network
let (mut network, mut events) = LibP2PNetwork::new(config, None)?;

// Bootstrap DHT
network.bootstrap_dht()?;

// Broadcast a block
network.broadcast_block(block)?;

// Process events
tokio::spawn(async move {
    while let Some(event) = events.recv().await {
        match event {
            NetworkEvent::BlockReceived { from, block } => {
                // Handle received block
            }
            NetworkEvent::PeerConnected { peer_id, addresses } => {
                // Handle new peer connection
            }
            _ => {}
        }
    }
});

// Run the network event loop
network.run().await;
```

### HTTP P2P Network

```rust
use ippan_p2p::{P2PConfig, HttpP2PNetwork};

let config = P2PConfig {
    listen_address: "http://0.0.0.0:9000".to_string(),
    bootstrap_peers: vec!["http://bootstrap-node:9000".to_string()],
    enable_upnp: true,
    ..Default::default()
};

let mut network = HttpP2PNetwork::new(config, "http://0.0.0.0:9000".to_string())?;
network.start().await?;
```

## Configuration

### LibP2PConfig
- `listen_addresses`: Multiaddrs to listen on (default: IPv4 and IPv6 TCP on random port)
- `bootstrap_peers`: Initial peers to connect to for DHT bootstrap
- `enable_mdns`: Enable local peer discovery (default: true)
- `enable_relay`: Enable relay protocol for NAT traversal (default: true)
- `idle_connection_timeout`: Time before idle connections are closed (default: 60s)

### P2PConfig (HTTP)
- `listen_address`: HTTP address to listen on
- `bootstrap_peers`: List of known peer addresses
- `max_peers`: Maximum number of concurrent peers
- `peer_discovery_interval`: How often to query peers for more peers
- `enable_upnp`: Enable UPnP port mapping
- `external_ip_services`: Services to query for external IP

## Network Events

### libp2p Events
- `PeerConnected`: New peer discovered and connected
- `PeerDisconnected`: Peer connection closed
- `BlockReceived`: Block received via gossipsub
- `TransactionReceived`: Transaction received via gossipsub
- `DhtQueryComplete`: DHT query completed with peer results

### HTTP Events
- `Block`: Block received from peer
- `Transaction`: Transaction received from peer
- `PeerInfo`: Peer announced its information
- `PeerDiscovery`: Discovered new peers from a peer
- `BlockRequest`: Peer requested a block
- `BlockResponse`: Response to block request

## NAT Traversal

The libp2p implementation includes comprehensive NAT traversal:

1. **Direct Connection**: Attempts direct connection first
2. **mDNS**: Discovers peers on local network automatically
3. **Relay Protocol**: Routes connections through relay nodes when direct connection fails
4. **DCUtR (Direct Connection Upgrade through Relay)**: Attempts to upgrade relayed connections to direct connections using hole punching

## Integration Notes

- **Start the network before syncing DAG state** to ensure peer connectivity
- **Use `NetworkMessage` enums** to exchange blocks and transactions over HTTP endpoints (legacy)
- **Combine with `ippan_rpc`** to expose network stats and respond to discovery requests
- **The libp2p implementation is recommended for production** deployments
- **HTTP implementation is suitable for development/testing** environments

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     IPPAN P2P Stack                         │
├─────────────────────────────────────────────────────────────┤
│  libp2p Network (Production)   │  HTTP Network (Legacy)    │
├────────────────────────────────┼───────────────────────────┤
│  • Kademlia DHT                │  • HTTP REST API          │
│  • GossipSub                   │  • UPnP Port Mapping      │
│  • mDNS Discovery              │  • External IP Detection  │
│  • Relay + DCUtR               │  • Peer Metadata          │
│  • Request/Response            │  • Message Broadcasting   │
│  • Identify Protocol           │  • Simple Peer Discovery  │
└────────────────────────────────┴───────────────────────────┘
                         │
                         ▼
              ┌───────────────────┐
              │  Parallel Gossip  │
              │   (Local Layer)   │
              └───────────────────┘
                         │
                         ▼
              ┌───────────────────┐
              │    IPPAN Types    │
              │ (Block/Transaction)│
              └───────────────────┘
```

## Testing

The crate includes comprehensive unit tests for both implementations:

```bash
# Run all tests
cargo test -p ippan-p2p

# Run only libp2p tests
cargo test -p ippan-p2p libp2p_network

# Run only HTTP tests
cargo test -p ippan-p2p http
```

## Performance Considerations

- **GossipSub** uses message deduplication and efficient routing
- **Kademlia DHT** provides O(log n) peer discovery
- **Connection pooling** reduces connection overhead
- **mDNS** provides instant local peer discovery with zero configuration
- **Relay connections** have higher latency than direct connections; DCUtR attempts to upgrade them

## Security

- **All messages are authenticated** using peer identities (libp2p)
- **GossipSub message validation** prevents spam and invalid messages
- **Connection limits** prevent resource exhaustion
- **NAT traversal is done securely** using cryptographic identities
- **TLS-like encryption** via Noise protocol (libp2p)

## Future Enhancements

- [ ] QUIC transport for improved performance
- [ ] AutoNAT for automatic NAT detection
- [ ] Circuit Relay v2 with reservation
- [ ] Enhanced peer scoring and reputation
- [ ] Bandwidth limiting and QoS
- [ ] Custom DHT record storage
- [ ] Prometheus metrics export
- [ ] Connection multiplexing optimization

## Dependencies

- `libp2p`: Core P2P networking (v0.56)
- `tokio`: Async runtime
- `serde`: Serialization
- `parking_lot`: Efficient synchronization
- `igd`: UPnP support (HTTP implementation)
- `reqwest`: HTTP client (HTTP implementation)

## License

Apache-2.0

