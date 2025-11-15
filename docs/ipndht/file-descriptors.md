## IPNDHT File Descriptors

The IPNDHT-backed file registry exposes metadata only – it does **not** move bulk
data. Nodes mint deterministic descriptors locally, publish them to the DHT, and
serve them over the RPC API for explorers and auxiliary services.

### Canonical Descriptor

`FileDescriptor` instances live in `ippan-types` and contain:

| Field | Type | Notes |
| --- | --- | --- |
| `id` | `FileDescriptorId` | `blake3("ipn-file-descriptor" ∥ HashTimer.digest ∥ content_hash ∥ owner)` |
| `content_hash` | `[u8; 32]` | Hex-encoded in JSON (BLAKE3/other) |
| `owner` | `Address` | Base58Check address owning the metadata |
| `size_bytes` | `u64` | Logical file size |
| `created_at` | `HashTimer` | Derived with domain `files/publish` |
| `mime_type` | `Option<String>` | Trimmed, limited to 128 chars |
| `tags` | `Vec<String>` | Normalized, ≤16 tags, ≤64 chars each |

Storage enforces uniqueness per `id`, keeps `id → descriptor` plus an owner index.

### DHT Integration

`IpnDhtService` caches descriptors locally and, when a libp2p stack is configured,
stores the entire descriptor as a Kademlia record under the descriptor ID.

- `publish_file`: caches + `put_record` + `start_providing`.
- `find_file`: returns cached entries or issues `get_record`, caching the result.

The libp2p network gained primitive DHT commands (`PutRecord`, `GetRecord`,
`StartProviding`, `GetProviders`) wired through the swarm event loop.

### RPC Endpoints

`POST /files/publish`

```json
{
  "owner": "i....",
  "content_hash": "ff00...00",
  "size_bytes": 1048576,
  "mime_type": "application/octet-stream",
  "tags": ["archive", "snapshot"]
}
```

Returns:

```json
{
  "descriptor": {
    "id": "4ba3...",
    "content_hash": "ff00...00",
    "owner": "i....",
    "size_bytes": 1048576,
    "created_at": { "...": "HashTimer payload ..." },
    "mime_type": "application/octet-stream",
    "tags": ["archive", "snapshot"]
  }
}
```

`GET /files/{id}`

1. Reads from the local registry.
2. Falls back to `IpnDhtService::find_file` if missing locally.
3. Returns 404 if neither source finds a descriptor.

A successful lookup returns the same payload as the publish endpoint.

### Usage Notes

- Descriptors are **off-chain** today. They can be minted without a transaction.
- HashTimer inputs include the node ID, so replays naturally produce distinct
  IDs unless both the content hash and creation time collide.
- Consumers should treat descriptors as hints about where to fetch actual
  content; the metadata is authoritative and verifiable via the `id`.
