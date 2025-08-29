# IPPAN Blockchain - Comprehensive Test Plan

## Project Overview
IPPAN is a high-performance blockchain designed to achieve **10M TPS** throughput with the following key components:
- **Node HTTP API** (Axum-based)
- **Wallet CLI** (Ed25519 cryptography)
- **Load Generator** (Performance testing)
- **P2P Networking** (libp2p)
- **Blockchain Core** (Mempool, Blocks, Rounds)

## Test Categories

### 1. Unit Tests

#### 1.1 Common Crate Tests
- **Address Validation**: Test base58i address format validation
- **Transaction Creation**: Test transaction structure and serialization
- **Crypto Operations**: Test Ed25519 signing/verification and batch operations
- **HashTimer Generation**: Test deterministic hashtimer computation
- **Merkle Root**: Test Blake3 merkle tree computation
- **Time Synchronization**: Test IPPAN time with median-of-peers

#### 1.2 Node Crate Tests
- **HTTP API Endpoints**: Test health, metrics, and transaction endpoints
- **P2P Networking**: Test libp2p connection and message handling
- **Mempool Operations**: Test sharded priority queue operations
- **Block Building**: Test micro-block creation and validation
- **Round Management**: Test round progression and finality

#### 1.3 Wallet CLI Tests
- **Wallet Creation**: Test new wallet generation
- **Address Display**: Test base58i address formatting
- **Transaction Signing**: Test transaction creation and signing
- **Balance Management**: Test balance tracking and updates

#### 1.4 Load Generator Tests
- **Account Generation**: Test bulk account creation
- **Transaction Generation**: Test random transaction creation
- **Performance Metrics**: Test TPS calculation and reporting

### 2. Integration Tests

#### 2.1 End-to-End Transaction Flow
```
Test: Complete transaction lifecycle
Steps:
1. Create wallet using wallet-cli
2. Submit transaction via HTTP API
3. Verify transaction appears in mempool
4. Wait for block creation
5. Verify transaction finalization
6. Check balance updates
```

#### 2.2 Multi-Node Network
```
Test: P2P network communication
Steps:
1. Start multiple nodes
2. Verify peer discovery via mDNS/Kademlia
3. Test transaction propagation via gossipsub
4. Verify consensus across nodes
5. Test network partition recovery
```

#### 2.3 Load Testing Scenarios
```
Test: High-throughput scenarios
Scenarios:
- 1K TPS for 60 seconds
- 10K TPS for 30 seconds
- 100K TPS for 10 seconds
- 1M TPS for 5 seconds
- Burst testing (0 to 1M TPS)
```

### 3. Performance Tests

#### 3.1 Throughput Benchmarks
- **Target**: 10M TPS sustained for 300 seconds
- **Measurement**: Transactions per second over time
- **Success Criteria**: Average TPS ≥ 10M, no degradation > 5%

#### 3.2 Latency Benchmarks
- **Target**: p50 ≤ 350ms, p95 ≤ 600ms
- **Measurement**: Transaction submission to finalization time
- **Success Criteria**: 95th percentile ≤ 600ms

#### 3.3 Memory Usage
- **Target**: Efficient memory usage under load
- **Measurement**: Memory consumption during high TPS
- **Success Criteria**: No memory leaks, stable usage

#### 3.4 CPU Utilization
- **Target**: Efficient CPU usage
- **Measurement**: CPU usage during load testing
- **Success Criteria**: < 80% CPU usage at 10M TPS

### 4. Security Tests

#### 4.1 Cryptographic Security
- **Ed25519 Verification**: Test signature verification
- **Batch Verification**: Test performance and correctness
- **Key Generation**: Test secure random key generation

#### 4.2 Network Security
- **Message Validation**: Test malformed message handling
- **Peer Authentication**: Test peer identity verification
- **DoS Protection**: Test against denial-of-service attacks

#### 4.3 Transaction Security
- **Double Spending**: Test prevention of double spending
- **Nonce Validation**: Test transaction ordering
- **Amount Validation**: Test overflow/underflow protection

### 5. Stress Tests

#### 5.1 Network Stress
- **Peer Disconnection**: Test behavior when peers disconnect
- **Message Flooding**: Test handling of message floods
- **Network Partition**: Test consensus during partitions

#### 5.2 Resource Stress
- **Memory Pressure**: Test under low memory conditions
- **CPU Pressure**: Test under high CPU load
- **Disk Pressure**: Test under slow disk conditions

#### 5.3 Load Stress
- **Peak Load**: Test beyond 10M TPS
- **Sustained Load**: Test 10M TPS for extended periods
- **Load Variations**: Test varying load patterns

### 6. API Tests

#### 6.1 HTTP API Endpoints
```bash
# Health endpoint
curl -X GET http://localhost:8080/health
Expected: {"status":"healthy","peers":0,"mempool_size":0}

# Metrics endpoint
curl -X GET http://localhost:8080/metrics
Expected: Prometheus-formatted metrics

# Transaction submission
curl -X POST http://localhost:8080/tx \
  -H "Content-Type: application/octet-stream" \
  -d @transaction.bin
Expected: {"status":"accepted","tx_id":"..."}
```

#### 6.2 Error Handling
- **Invalid Transactions**: Test rejection of malformed transactions
- **Rate Limiting**: Test API rate limiting
- **Authentication**: Test if authentication is required

### 7. CLI Tests

#### 7.1 Wallet CLI Commands
```bash
# Create wallet
cargo run -p ippan-wallet-cli -- new testwallet
Expected: Wallet created successfully

# Show address
cargo run -p ippan-wallet-cli -- addr
Expected: i1abc123... (base58i address)

# Send transaction
cargo run -p ippan-wallet-cli -- send --to i1def456... --amount 1000 --node http://localhost:8080
Expected: Transaction sent successfully
```

#### 7.2 Load Generator Commands
```bash
# Start load test
cargo run -p ippan-loadgen-cli -- --tps 1000 --accounts 100 --duration 10 --nodes http://localhost:8080
Expected: Load test completed with success rate > 95%
```

### 8. Benchmark Tests

#### 8.1 Criterion Benchmarks
```bash
# Run all benchmarks
cargo bench -p ippan-bench

# Specific benchmarks
cargo bench -p ippan-bench ed25519_batch_verify
cargo bench -p ippan-bench mempool_enqueue
cargo bench -p ippan-bench block_build
```

#### 8.2 Performance Targets
- **Ed25519 Batch Verify**: > 100K signatures/second
- **Mempool Enqueue**: > 1M transactions/second
- **Block Build**: < 10ms for 16KB blocks
- **Merkle Root**: < 1ms for 1000 transactions

### 9. Monitoring Tests

#### 9.1 Metrics Collection
- **Prometheus Metrics**: Verify all metrics are exposed
- **Metric Accuracy**: Verify metric values are correct
- **Metric Cardinality**: Verify no metric explosion

#### 9.2 Logging
- **Log Levels**: Test different log levels
- **Log Content**: Verify log messages are informative
- **Log Performance**: Test logging doesn't impact performance

### 10. Deployment Tests

#### 10.1 Build Tests
```bash
# Release build
cargo build --release
Expected: Successful build with optimizations

# Debug build
cargo build
Expected: Successful build with debug info
```

#### 10.2 Configuration Tests
- **CLI Arguments**: Test all command-line options
- **Environment Variables**: Test environment-based configuration
- **Config Files**: Test configuration file loading

## Test Execution Plan

### Phase 1: Unit Tests (Day 1-2)
- Run all unit tests
- Fix any failing tests
- Achieve 100% test coverage

### Phase 2: Integration Tests (Day 3-4)
- Set up multi-node test environment
- Run end-to-end transaction tests
- Test P2P networking

### Phase 3: Performance Tests (Day 5-7)
- Run throughput benchmarks
- Run latency benchmarks
- Optimize performance bottlenecks

### Phase 4: Stress Tests (Day 8-9)
- Run stress tests
- Test failure scenarios
- Verify system resilience

### Phase 5: Security Tests (Day 10)
- Run security tests
- Verify cryptographic operations
- Test attack scenarios

## Success Criteria

### Functional Success
- All unit tests pass
- All integration tests pass
- All API endpoints work correctly
- All CLI commands work correctly

### Performance Success
- Achieve 10M TPS sustained for 300 seconds
- Maintain p50 ≤ 350ms, p95 ≤ 600ms latency
- CPU usage < 80% at peak load
- Memory usage stable with no leaks

### Security Success
- All cryptographic operations verified
- No security vulnerabilities found
- Proper error handling for malicious inputs

### Reliability Success
- System handles network partitions
- System recovers from failures
- No data loss during stress tests

## Test Environment Requirements

### Hardware Requirements
- **CPU**: 32+ cores recommended
- **RAM**: 64GB+ recommended
- **Network**: 10Gbps+ recommended
- **Storage**: SSD with 1TB+ space

### Software Requirements
- **OS**: Linux (Ubuntu 20.04+)
- **Rust**: 1.70+ stable
- **Docker**: For containerized testing
- **Monitoring**: Prometheus + Grafana

### Network Requirements
- **Bandwidth**: 10Gbps+ for high-throughput tests
- **Latency**: < 1ms for local tests
- **Isolation**: Dedicated network for testing

## Reporting

### Test Reports
- Daily test execution reports
- Performance benchmark reports
- Security test reports
- Bug reports with reproduction steps

### Metrics Dashboard
- Real-time performance metrics
- System resource utilization
- Network statistics
- Error rates and types

### Documentation
- Test execution logs
- Performance analysis
- Security assessment
- Recommendations for improvements
