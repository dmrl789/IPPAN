# IPNDHT File Publishing and Lookup

## Overview

The IPPAN file descriptor system provides metadata tracking and DHT-based discovery for content-addressed files. This is **metadata only** — actual file content is not stored or transferred.

## Architecture

### Components

1. **FileDescriptor** - Metadata record containing:
   - `id`: Unique HashTimer-based identifier
   - `content_hash`: BLAKE3 hash of file content
   - `owner`: 32-byte address
   - `size_bytes`: File size (u64)
   - `created_at_us`: Creation timestamp (microseconds)
   - `mime_type`: Optional MIME type
   - `tags`: Optional categorization tags

2. **FileStorage** - Local indexing:
   - Primary index: `file_id → FileDescriptor`
   - Secondary index: `owner → [FileId]`
   - Time-ordered index for pagination

3. **FileDhtService** - DHT integration:
   - `publish_file`: Publish descriptor to Kademlia DHT
   - `find_file`: Query DHT for descriptor by ID

## ID Generation

File IDs are deterministic and time-ordered:

```
ID = HashTimer.derive(
    context = "file",
    time = now(),
    domain = content_hash,
    payload = owner,
    nonce = [0; 32],  // deterministic
    node_id = owner
)
```

This ensures:
- **Uniqueness**: Different content or owners produce different IDs
- **Determinism**: Same inputs always produce the same ID
- **Time-ordering**: IDs can be sorted chronologically
- **No collisions**: HashTimer digest provides 256-bit security

## RPC Endpoints

### POST /files/publish

Publish a new file descriptor.

**Request:**
```json
{
  "owner": "ippan1...",
  "content_hash": "0123456789abcdef...",
  "size_bytes": 1024,
  "mime_type": "text/plain",
  "tags": ["document", "public"]
}
```

**Response:**
```json
{
  "id": "a1b2c3d4...",
  "content_hash": "0123456789abcdef...",
  "owner": "ippan1...",
  "size_bytes": 1024,
  "created_at_us": 1700000000000000,
  "mime_type": "text/plain",
  "tags": ["document", "public"],
  "dht_published": true
}
```

**Behavior:**
1. Validates owner address and content hash format
2. Creates deterministic FileDescriptor with HashTimer-based ID
3. Stores locally in canonical storage
4. Publishes to IPNDHT for peer discovery
5. Returns descriptor with generated ID

### GET /files/{id}

Lookup a file descriptor by ID.

**Request:**
```
GET /files/a1b2c3d4e5f6...
```

**Response (found):**
```json
{
  "id": "a1b2c3d4...",
  "content_hash": "0123456789abcdef...",
  "owner": "ippan1...",
  "size_bytes": 1024,
  "created_at_us": 1700000000000000,
  "mime_type": "text/plain",
  "tags": ["document", "public"]
}
```

**Response (not found):**
```json
{
  "code": "not_found",
  "message": "File descriptor not found"
}
```

**Behavior:**
1. Checks local storage first
2. Falls back to DHT query if not found locally
3. Returns 404 if not found anywhere

## Data Types

### FileId

64-character hex string (32 bytes):
- Derived from HashTimer digest
- Time-ordered and deterministic
- Example: `a1b2c3d4e5f6789...` (64 chars)

### ContentHash

64-character hex string (32 bytes):
- BLAKE3 hash of file content
- Deterministic for same content
- Example: `0123456789abcdef...` (64 chars)

### Owner Address

Base58Check-encoded IPPAN address:
- Format: `ippan1...`
- 32-byte address internally
- Ed25519 public key hash

## Implementation Notes

### Current Limitations

1. **DHT Integration**: Initial version uses stub implementation
   - Actual libp2p Kademlia integration is placeholder
   - All DHT operations are marked as "stub" in logs

2. **File Storage**: Not distributed
   - Each node maintains local index
   - No automatic replication (yet)

3. **Content Storage**: Not implemented
   - Only metadata is tracked
   - Actual file content must be stored separately

### Storage Guarantees

- **Atomic writes**: Descriptor storage is transactional
- **No partial state**: Failed operations leave no side effects
- **Uniqueness**: File IDs are enforced unique (deterministic)
- **Validation**: All descriptors validated before storage

### Security

- Rate limiting applies to all `/files/*` endpoints
- Circuit breaker protects against cascading failures
- Owner addresses are validated (base58check or hex)
- Content hashes must be exactly 64 hex characters

## Example Workflow

### Publishing a File

1. Application computes BLAKE3 hash of file content:
   ```rust
   let content_hash = blake3::hash(&file_bytes);
   ```

2. Submit publish request:
   ```bash
   curl -X POST http://localhost:9000/files/publish \
     -H "Content-Type: application/json" \
     -d '{
       "owner": "ippan1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqwkl3a9",
       "content_hash": "..."64 hex chars...",
       "size_bytes": 1024,
       "mime_type": "image/png"
     }'
   ```

3. Server responds with generated ID:
   ```json
   {
     "id": "a1b2c3d4...",
     "dht_published": true,
     ...
   }
   ```

4. Store the ID for later lookup

### Looking Up a File

1. Query by ID:
   ```bash
   curl http://localhost:9000/files/a1b2c3d4e5f6...
   ```

2. Receive descriptor:
   ```json
   {
     "id": "a1b2c3d4...",
     "content_hash": "...",
     "owner": "ippan1...",
     "size_bytes": 1024,
     "mime_type": "image/png"
   }
   ```

3. Use content_hash to retrieve actual file from content storage (external)

## Future Enhancements

1. **Full libp2p DHT**:
   - Replace stub with actual Kademlia integration
   - Provider records for file announcements
   - Multi-hop lookups

2. **Replication**:
   - Automatic descriptor sync between nodes
   - Configurable replication factor

3. **Content Addressing**:
   - Integrate with IPFS or similar for content storage
   - Link descriptors to content providers

4. **Query Extensions**:
   - Search by owner
   - Filter by tags or MIME type
   - Range queries by creation time

5. **Pinning**:
   - Mark descriptors for persistent storage
   - Garbage collection for unpinned items

## Testing

Run tests:
```bash
# Test file descriptor model
cargo test -p ippan-files

# Test RPC handlers
cargo test -p ippan-rpc -- files

# Run all file-related tests
cargo test --workspace -- files
```

## Integration

To use file descriptors in your application:

```rust
use ippan_files::{FileDescriptor, ContentHash, MemoryFileStorage, StubFileDhtService};
use ippan_rpc::AppState;

// Create storage and DHT
let file_storage = Arc::new(MemoryFileStorage::new());
let file_dht = Arc::new(StubFileDhtService::new());

// Add to AppState
let state = AppState {
    // ... other fields ...
    file_storage: Some(file_storage),
    file_dht: Some(file_dht),
};

// Use via RPC endpoints or directly
```
