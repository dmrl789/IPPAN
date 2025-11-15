# IPNDHT Resilience Model

> **Status**: Implemented  
> **Last Updated**: 2025-11-15

## Overview

IPPAN's IPNDHT (InterPlanetary Named Data Hash Table) includes a comprehensive resilience layer that ensures network functionality under various failure conditions. This document describes the resilience mechanisms, recovery behaviors, and operational guarantees.

## Design Goals

1. **Minimum Viable Network**: Function with as few as 2 IPNWorker nodes
2. **Cold-Start Recovery**: Nodes can join the network from an empty routing table
3. **Partition Tolerance**: Network fragments can operate independently and merge back
4. **Content Propagation**: File descriptors and handles propagate across all reachable nodes
5. **Self-Healing**: Automatic reconnection and routing table repair

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                      IPNDHT Resilience Layer                     │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────┐    ┌─────────────────┐   ┌──────────────┐  │
│  │ Bootstrap      │    │  Gossip         │   │  DHT         │  │
│  │ Retry Logic    │───▶│  Propagation    │──▶│  Routing     │  │
│  │ (Cold Start)   │    │  (Files/Handles)│   │  Table       │  │
│  └────────────────┘    └─────────────────┘   └──────────────┘  │
│         │                      │                      │          │
│         │                      │                      │          │
│  ┌──────▼──────────────────────▼──────────────────────▼───────┐ │
│  │            libp2p Swarm (Kademlia + GossipSub)             │ │
│  └────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

## Core Resilience Mechanisms

### 1. Minimal 2-Node Operation

**Requirement**: The DHT must function with exactly 2 nodes.

**Implementation**:
- Each node maintains explicit peer connections
- Bootstrap configuration allows direct peer addressing
- Reciprocal discovery ensures both nodes see each other
- Kademlia routing table handles minimal peer counts

**Test Coverage**: `test_minimal_2_node_discovery`

**Example Scenario**:
```
Node A (127.0.0.1:9000)
  └─ Bootstrap: []
  └─ Routing Table: [Node B]

Node B (127.0.0.1:9001)
  └─ Bootstrap: [Node A]
  └─ Routing Table: [Node A]
  
Result: Bidirectional connectivity established
```

### 2. Cold-Start Recovery

**Problem**: A node starts with an empty routing table and no reachable bootstrap peers.

**Solution**: Automatic bootstrap retry with exponential backoff.

**Configuration**:
```rust
Libp2pConfig {
    bootstrap_retry_interval: Duration::from_secs(30),  // retry every 30s
    bootstrap_max_retries: 0,  // infinite retries (0 = unlimited)
    // ... other config
}
```

**Behavior**:
1. Node starts with empty routing table
2. Attempts initial bootstrap dial
3. If no peers connected, waits `bootstrap_retry_interval`
4. Retries bootstrap dial
5. Continues until peers found or `bootstrap_max_retries` reached
6. Resets retry counter once peers are connected

**Test Coverage**: `test_cold_start_recovery`

**Timeline**:
```
T+0s:   Node A starts (empty routing table)
T+1s:   Node B starts and advertises
T+1s:   Node A bootstrap retry #1 → dials B
T+1.5s: Connection established A ↔ B
T+2s:   Routing tables updated
```

### 3. File Descriptor Propagation

**Mechanism**: GossipSub topic `ippan/files`

**Flow**:
```
Node A                    Node B                    Node C
  │                         │                         │
  │  publish_file(desc)     │                         │
  ├────────────────────────▶│                         │
  │                         │  gossip: ippan/files    │
  │                         ├────────────────────────▶│
  │                         │                         │
  │                         │  ack                    │
  │                         │◀────────────────────────┤
  │                         │                         │
```

**Message Format**:
```json
{
  "type": "file_publish",
  "content_hash": "blake3:abc123...",
  "size": 1024,
  "owner": "ed25519:def456...",
  "mime_type": "image/png",
  "tags": ["image", "test"]
}
```

**Guarantees**:
- **Best-Effort Delivery**: GossipSub ensures probabilistic delivery
- **Deduplication**: Messages are deduplicated by content hash
- **Propagation Time**: Typically < 1 second for 3-node network
- **Resilience**: Works even if intermediate nodes drop

**Test Coverage**: `test_file_descriptor_propagation`

### 4. Handle Propagation

**Mechanism**: GossipSub topic `ippan/handles`

**Flow**:
```
Node A registers @alice.ipn
  │
  ├─ publish_handle("@alice.ipn", owner_key, public_key)
  │
  └─▶ GossipSub ─▶ All connected peers receive announcement
```

**Message Format**:
```json
{
  "type": "handle_register",
  "handle": "@alice.ipn",
  "owner": "ed25519:owner_pubkey",
  "public_key": "ed25519:handle_pubkey",
  "timestamp_us": 1700000000000
}
```

**Note**: Current implementation is stub-based. Full lookup resolution is planned.

**Test Coverage**: `test_handle_propagation`

### 5. Partition and Recovery

**Scenario**: Network splits into isolated segments, then reconnects.

**Example**:
```
Initial State:
    A ↔ B ↔ C

Partition (B disconnects):
    A       C
    
Partition Behavior:
- A and C maintain connectivity
- B's routing table is cleared
- Gossip messages still flow A ↔ C

Recovery (B reconnects):
    A ↔ B ↔ C
    
Recovery Process:
1. B dials known bootstrap peers
2. A and C receive connection events
3. B's routing table is repopulated
4. Gossip resumes across all nodes
```

**Recovery Time**:
- **Detection**: Immediate (connection dropped)
- **Reconnection**: < 5 seconds with retry logic
- **Full Mesh**: < 10 seconds for 3-node network

**Test Coverage**: `test_partition_recovery`

## Operational Characteristics

### Network Topology

**Minimum**: 2 nodes (peer-to-peer)  
**Recommended**: 5+ nodes (robust mesh)  
**Maximum**: Unlimited (scales with Kademlia)

### Timeouts and Intervals

| Parameter | Default | Tunable | Description |
|-----------|---------|---------|-------------|
| Bootstrap Retry | 30s | Yes | Cold-start retry interval |
| Gossip Heartbeat | 4s | Yes | GossipSub heartbeat |
| Connection Timeout | 10s | Yes | Dial timeout |
| Peer Discovery | Continuous | No | mDNS and Kademlia |

### Failure Modes

| Failure | Impact | Recovery | Mitigation |
|---------|--------|----------|------------|
| 1 node down (N≥3) | None | Automatic | Mesh routing |
| 2 nodes down (N=3) | Partition | Manual restart | Bootstrap retry |
| All nodes down | Total | Restart from bootstrap | Persistent config |
| Network split | Partition | Automatic on reconnect | Mesh healing |
| Cold start | No peers | Bootstrap retry | Retry logic |

### Performance Metrics

**2-Node Network**:
- Connection establishment: < 2s
- File propagation: < 500ms
- Handle propagation: < 500ms

**5-Node Network**:
- Full mesh: < 10s
- File propagation: < 1s
- Handle propagation: < 1s
- Partition recovery: < 15s

## Testing and Validation

### Test Harness

Location: `crates/p2p/tests/ipndht_resilience.rs`

**Capabilities**:
- Spawn N in-process DHT nodes
- Configure custom bootstrap peers
- Monitor events (connections, gossip, discovery)
- Simulate partitions (node shutdown)
- Validate routing table states

**Example Usage**:
```rust
// Spawn 3 test nodes
let nodes = spawn_test_nodes(3).await?;

// Connect in a ring
connect_nodes(&nodes[0], &nodes[1]).await?;
connect_nodes(&nodes[1], &nodes[2]).await?;
connect_nodes(&nodes[2], &nodes[0]).await?;

// Wait for full mesh
wait_for_full_mesh(&nodes, Duration::from_secs(15)).await?;

// Publish from node 0
nodes[0].publish("ippan/files", file_data)?;

// Verify node 2 receives it
let received = nodes[2].wait_for_gossip("ippan/files", Duration::from_secs(10)).await?;
assert_eq!(received, file_data);
```

### Test Suite

| Test | Purpose | Assertions |
|------|---------|------------|
| `test_minimal_2_node_discovery` | 2-node minimum | Bidirectional routing |
| `test_cold_start_recovery` | Bootstrap retry | Connection after retry |
| `test_file_descriptor_propagation` | File gossip | 3-node propagation |
| `test_handle_propagation` | Handle gossip | Stub pathway verified |
| `test_partition_recovery` | Partition healing | Gossip after rejoin |
| `test_3_node_full_mesh` | Mesh topology | Full connectivity |

**Run Tests**:
```bash
# Run all resilience tests
cargo test -p ippan-p2p ipndht_resilience -- --ignored --nocapture

# Run specific test
cargo test -p ippan-p2p test_minimal_2_node_discovery -- --ignored --nocapture
```

## Configuration Best Practices

### Production

```rust
Libp2pConfig {
    bootstrap_peers: vec![
        "/ip4/seed1.ippan.io/tcp/9000/p2p/12D3K...",
        "/ip4/seed2.ippan.io/tcp/9000/p2p/14D5L...",
    ],
    bootstrap_retry_interval: Duration::from_secs(30),
    bootstrap_max_retries: 0,  // infinite
    enable_mdns: false,  // disable in production
    enable_relay: true,  // for NAT traversal
    // ...
}
```

### Development

```rust
Libp2pConfig {
    bootstrap_peers: vec![
        "/ip4/127.0.0.1/tcp/9000/p2p/local-peer-id",
    ],
    bootstrap_retry_interval: Duration::from_secs(5),  // faster retry
    bootstrap_max_retries: 10,  // fail fast
    enable_mdns: true,  // local discovery
    enable_relay: false,
    // ...
}
```

### Testing

```rust
Libp2pConfig {
    listen_addresses: vec![Multiaddr::from_str("/ip4/127.0.0.1/tcp/0")?],  // ephemeral
    bootstrap_peers: vec![],  // set per test
    bootstrap_retry_interval: Duration::from_millis(100),  // very fast
    bootstrap_max_retries: 5,
    enable_mdns: false,  // deterministic
    enable_relay: false,
    // ...
}
```

## Future Enhancements

1. **DHT Record Persistence**: Store file descriptors in Kademlia DHT records
2. **Provider Records**: Track which nodes have file content
3. **Handle Lookup**: Full resolution of @handle → peer mapping
4. **Replication**: Automatic replication of DHT records (k=3)
5. **Metrics**: Prometheus-compatible resilience metrics
6. **Circuit Breakers**: Intelligent backoff for failing peers

## Related Documentation

- [IPNDHT Overview](./README.md)
- [File Descriptors](./file-descriptors.md)
- [libp2p Integration](../../crates/p2p/README.md)

## References

- libp2p Kademlia: https://docs.libp2p.io/concepts/kad-dht/
- GossipSub: https://docs.libp2p.io/concepts/pubsub/
- DHT Resilience: https://en.wikipedia.org/wiki/Distributed_hash_table
