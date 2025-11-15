# IPNDHT Documentation

IPPAN's IPNDHT (InterPlanetary Named Data Hash Table) system provides content-addressed file discovery and peer-to-peer metadata distribution.

## Core Documents

- [File Descriptors](./file-descriptors.md) - Metadata tracking and DHT-based file publishing/lookup

## Overview

IPNDHT extends IPPAN's DHT capabilities with:

1. **Content Addressing**: Files identified by BLAKE3 hashes
2. **Metadata Distribution**: File descriptors published via Kademlia DHT
3. **Decentralized Discovery**: Nodes can find file metadata without central registry
4. **Time-Ordered IDs**: HashTimer-based identifiers for chronological sorting

## Architecture

```
┌─────────────────┐
│  RPC Endpoints  │ POST /files/publish, GET /files/{id}
└────────┬────────┘
         │
┌────────▼────────┐
│ FileDescriptor  │ ID, content_hash, owner, size, metadata
└────────┬────────┘
         │
    ┌────┴────┐
    │         │
┌───▼───┐ ┌──▼──┐
│Storage│ │ DHT │ Local index + Kademlia distribution
└───────┘ └─────┘
```

## Status

- **Phase 1** (Current): Metadata tracking, local storage, stub DHT
- **Phase 2** (Planned): Full Kademlia integration with libp2p
- **Phase 3** (Planned): Content storage integration (IPFS/similar)

## See Also

- [File Descriptor API](./file-descriptors.md#rpc-endpoints)
- [ID Generation](./file-descriptors.md#id-generation)
- [Integration Guide](./file-descriptors.md#integration)
