# IPPAN P2P

## Overview
- Implements the HTTP-based P2P network manager used by node and RPC services.
- Handles peer discovery, announcements, message broadcasting, and NAT traversal.
- Leverages the `ippan_network` primitives while adding transport orchestration.

## Key Modules
- `parallel_gossip`: deterministic gossip harness re-exported for downstream crates.
- `lib.rs`: HTTP P2P manager with peer metadata, message queues, and request handling.
- Built-in support for UPnP, external IP detection, and peer health tracking.

## Integration Notes
- Start `HttpP2PNetwork` with the desired listen address and bootstrap peers before syncing DAG state.
- Use `NetworkMessage` enums to exchange blocks, transactions, and peer info over HTTP endpoints.
- Combine with `ippan_rpc` to expose network stats and respond to peer discovery requests.
