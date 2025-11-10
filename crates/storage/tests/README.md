# Storage Integration Tests

This directory contains comprehensive integration tests for the IPPAN storage layer, covering both the Sled-backed persistent storage and the in-memory mock storage implementations.

## Test Coverage

### Storage Backends Tested

1. **MemoryStorage** - In-memory implementation for testing
2. **SledStorage** - Persistent embedded database backend

### Functional Areas Covered

The test suite validates all storage operations defined in the `Storage` trait:

#### 1. Block Storage (`test_block_storage`)
- Storing and retrieving blocks by hash
- Retrieving blocks by height/round
- Tracking latest height
- Parent-child block relationships

#### 2. Transaction Storage (`test_transaction_storage`)
- Storing and retrieving transactions by hash
- Getting transaction count
- Querying transactions by address (sender/recipient)
- Multi-address transaction filtering

#### 3. Account Management (`test_account_storage`)
- Creating and updating accounts
- Retrieving accounts by address
- Listing all accounts
- Updating balances and nonces
- Non-existent account handling

#### 4. L2 Network Operations (`test_l2_network_storage`)
- Registering L2 networks
- Retrieving networks by ID
- Listing all registered networks
- Network status tracking

#### 5. L2 Commit Storage (`test_l2_commit_storage`)
- Storing L2 state commitments
- Listing all commits
- Filtering commits by L2 network
- Epoch and state root tracking

#### 6. L2 Exit Records (`test_l2_exit_storage`)
- Recording exit requests
- Listing all exits
- Filtering by L2 network
- Exit status tracking

#### 7. Round Certificates (`test_round_certificate_storage`)
- Storing consensus round certificates
- Retrieving certificates by round
- Block ID aggregation

#### 8. Round Finalization (`test_round_finalization_storage`)
- Recording round finalizations
- Retrieving by round ID
- Tracking latest finalized round
- State root validation

#### 9. Chain State (`test_chain_state_storage`)
- Reading and updating chain state
- Total issuance tracking
- Round progression tracking
- DAG-Fair emission metadata

#### 10. Validator Telemetry (`test_validator_telemetry_storage`)
- Storing validator performance metrics
- Retrieving individual validator data
- Querying all validators
- AI consensus integration data

### Special Test Cases

#### Persistence Tests (`sled_storage_persistence`)
- Data survives database reopening
- Flush and reload operations
- Cross-session data integrity

#### Initialization Tests (`sled_storage_initialization`)
- Genesis block creation
- Genesis account setup
- First-run initialization

#### Stress Tests
- **Multiple Blocks** - Tests storage of 10 sequential blocks
- **Large Transaction Set** - Validates 100 transactions with address filtering
- **Concurrent Access** - Multi-threaded writes to shared storage

## Running Tests

### Run all storage tests:
```bash
cargo test -p ippan-storage
```

### Run only integration tests:
```bash
cargo test -p ippan-storage --test integration_tests
```

### Run specific test:
```bash
cargo test -p ippan-storage --test integration_tests memory_storage_blocks
```

### Run with output:
```bash
cargo test -p ippan-storage -- --nocapture
```

## Test Statistics

- **Total Integration Tests**: 25
- **Lines of Test Code**: 680
- **Storage Operations Tested**: All `Storage` trait methods
- **Backend Coverage**: 100% (both MemoryStorage and SledStorage)

## Test Architecture

Each functional area has a generic test function that accepts any `Storage` implementation:

```rust
fn test_block_storage<S: Storage>(storage: &S) {
    // Test implementation
}
```

This approach ensures both backends behave identically and maintain API compatibility.

### Test Helpers

The test suite includes helper functions for creating test data:
- `create_test_block()` - Creates blocks with proper round/parent relationships
- `create_test_transaction()` - Creates valid transactions with amounts and nonces
- `create_test_account()` - Creates accounts with balances
- `create_test_l2_network()` - Creates L2 network registrations
- `create_test_l2_commit()` - Creates L2 commitment records
- `create_test_l2_exit()` - Creates exit records
- `create_test_round_cert()` - Creates round certificates
- `create_test_round_finalization()` - Creates finalization records
- `create_test_validator_telemetry()` - Creates validator metrics

## Integration with IPPAN Types

Tests validate proper integration with core IPPAN types:
- `Block` - Round-scoped DAG blocks with HashTimer
- `Transaction` - Ed25519-signed transactions with confidential envelope support
- `L2Network` - L2 rollup metadata
- `L2Commit` - State commitments with ZK proofs
- `L2ExitRecord` - Exit requests with inclusion proofs
- `RoundCertificate` - Consensus attestations
- `RoundFinalizationRecord` - Execution records with state roots
- `ValidatorTelemetry` - AI consensus metrics

## Future Enhancements

Potential areas for expansion:
- [ ] Benchmark tests for performance validation
- [ ] Fuzz testing for edge cases
- [ ] Property-based testing with proptest
- [ ] Transaction ordering tests
- [ ] Snapshot/restore tests
- [ ] Migration tests for schema changes
