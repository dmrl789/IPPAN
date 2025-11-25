# Developer Guide: Using the New Signing & Verification APIs

**Quick Reference for IPPAN Developers**

---

## 1. Network Protocol Messages

### Signing a Message
```rust
use ippan_network::protocol::{NetworkMessage, MessageType};
use ed25519_dalek::SigningKey;

// Create a message
let mut message = NetworkMessage::new(
    MessageType::TransactionAnnouncement,
    "my-node-id".to_string(),
    payload_bytes,
);

// Sign with your private key (32 bytes)
let private_key: [u8; 32] = /* your key */;
message.sign(&private_key)?;

// Send the signed message
network.send(message).await?;
```

### Verifying a Message
```rust
// Receive a message
let message: NetworkMessage = /* from network */;

// Verify with sender's public key (32 bytes)
let public_key: [u8; 32] = peer_public_key;
if message.verify_signature(&public_key) {
    // Signature valid - process message
    handle_message(message)?;
} else {
    // Signature invalid - reject
    return Err("Invalid signature");
}
```

---

## 2. Handle Registry Operations

### Register a Handle
```rust
use ippan_l2_handle_registry::types::{Handle, HandleRegistration, PublicKey};
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};

let handle = Handle::new("@alice.ipn");
let signing_key = SigningKey::from_bytes(&your_private_key);
let owner = PublicKey::new(signing_key.verifying_key().to_bytes());

// Construct message to sign
let mut message = Vec::new();
message.extend_from_slice(b"IPPAN_HANDLE_REGISTRATION");
message.extend_from_slice(handle.as_str().as_bytes());
message.extend_from_slice(owner.as_bytes());
if let Some(expires) = expires_at {
    message.extend_from_slice(&expires.to_le_bytes());
}

// Sign the message
let message_hash = Sha256::digest(&message);
let signature = signing_key.sign(&message_hash);

// Create registration
let registration = HandleRegistration {
    handle,
    owner,
    signature: signature.to_bytes().to_vec(),
    metadata: HashMap::new(),
    expires_at: Some(1735689600), // Optional expiry
};

// Submit
registry.register(registration)?;
```

### Transfer a Handle
```rust
// Construct transfer message
let mut message = Vec::new();
message.extend_from_slice(b"IPPAN_HANDLE_TRANSFER");
message.extend_from_slice(handle.as_str().as_bytes());
message.extend_from_slice(from_owner.as_bytes());
message.extend_from_slice(to_owner.as_bytes());

// Sign with FROM owner's key
let message_hash = Sha256::digest(&message);
let signature = from_signing_key.sign(&message_hash);

let transfer = HandleTransfer {
    handle,
    from_owner,
    to_owner,
    signature: signature.to_bytes().to_vec(),
};

registry.transfer(transfer)?;
```

### Update Handle Metadata
```rust
// Construct update message
let mut message = Vec::new();
message.extend_from_slice(b"IPPAN_HANDLE_UPDATE");
message.extend_from_slice(handle.as_str().as_bytes());
message.extend_from_slice(owner.as_bytes());

// Sign
let message_hash = Sha256::digest(&message);
let signature = signing_key.sign(&message_hash);

let update = HandleUpdate {
    handle,
    owner,
    signature: signature.to_bytes().to_vec(),
    updates: metadata_changes,
};

registry.update(update)?;
```

---

## 3. Confidential Transactions

### Creating a Valid Confidential Transaction
```rust
use ippan_crypto::CryptoUtils;

let mut tx_data = Vec::new();

// Version
tx_data.push(1u8);

// Number of commitments (1-255)
tx_data.push(2u8); // 2 commitments

// Commitments (32 bytes each, must be non-zero)
tx_data.extend_from_slice(&commitment1); // [u8; 32], not all zeros
tx_data.extend_from_slice(&commitment2); // [u8; 32], not all zeros

// Range proof length (must be 1-10000)
let proof_len: u32 = range_proof.len() as u32;
tx_data.extend_from_slice(&proof_len.to_le_bytes());

// Range proof data
tx_data.extend_from_slice(&range_proof);

// Ed25519 signature (64 bytes, non-zero)
tx_data.extend_from_slice(&signature); // [u8; 64]

// Validate before submitting
if CryptoUtils::validate_confidential_transaction(&tx_data)? {
    submit_transaction(tx_data)?;
}
```

---

## 4. L1 Handle Ownership Proofs

### Creating an Ownership Proof
```rust
use ippan_l1_handle_anchors::anchors::L1HandleAnchorStorage;

let storage = L1HandleAnchorStorage::new();

// Store anchor first
storage.store_anchor(anchor)?;

// Generate proof (includes Merkle path)
let proof = storage.create_ownership_proof("@alice.ipn")?;

// Proof contains:
// - anchor (ownership data)
// - merkle_proof (Vec<[u8; 32]>) - path to root
// - state_root ([u8; 32]) - Merkle root
```

### Verifying an Ownership Proof
```rust
// Verify the proof
if storage.verify_ownership_proof(&proof) {
    // Proof valid
    let owner = proof.owner();
    let handle_hash = proof.handle_hash();
} else {
    // Proof invalid
    return Err("Invalid ownership proof");
}
```

---

## 5. AI Model Registration Fees

### Understanding Fee Calculation
```rust
use ippan_ai_registry::proposal::ProposalManager;

// Default fees
let manager = ProposalManager::default();
// base_fee = 1,000,000 µIPN (1 IPN)
// per_mb = 100,000 µIPN (0.1 IPN)

// Custom fees
let manager = ProposalManager::with_fees(
    0.67,           // voting_threshold
    1_000_000,      // min_proposal_stake
    2_000_000,      // base_registration_fee (2 IPN)
    200_000,        // fee_per_mb (0.2 IPN per MB)
);

// Fee is calculated automatically when executing proposal
let registration = manager.execute_proposal(proposal_id)?;
println!("Fee charged: {} µIPN", registration.registration_fee);
```

### Fee Examples
```
Model Size → Fee (default)
─────────────────────────
500 KB     → 1.1 IPN
1 MB       → 1.1 IPN
10 MB      → 2.0 IPN
50 MB      → 6.0 IPN
100 MB     → 11.0 IPN
1 GB       → 101.0 IPN
```

---

## 6. Economics Parameter Tracking

### Getting Current Parameters
```rust
use ippan_consensus::round_executor::RoundExecutor;

let executor = RoundExecutor::new(params, ledger);

// Get actual configured parameters (not defaults)
let current_params = executor.get_economics_params();
println!("Initial reward: {} µIPN", current_params.initial_round_reward_micro);
println!("Halving interval: {} rounds", current_params.halving_interval_rounds);
println!("Max supply: {} µIPN", current_params.max_supply_micro);
```

### Updating via Governance
```rust
// Governance decision to update parameters
let new_params = EmissionParams {
    initial_round_reward_micro: 5_000_000, // 5 IPN per round
    halving_interval_rounds: 105_120,      // ~2 years
    max_supply_micro: 21_000_000_000_000,  // 21M IPN
    ..Default::default()
};

executor.update_economics_params(new_params);

// Verify updated
assert_eq!(
    executor.get_economics_params().initial_round_reward_micro,
    5_000_000
);
```

---

## 7. Audit Checkpoint Fees

### Understanding Fee Tracking
```rust
use ippan_consensus::emission_tracker::EmissionTracker;

let mut tracker = EmissionTracker::new(params, 1000);

// Process rounds - fees automatically tracked
for round in 1..=1000 {
    tracker.process_round(
        round,
        &contributions,
        transaction_fees,  // ← Accumulated per period
        ai_commissions,
    )?;
}

// Audit checkpoint created every 1000 rounds
let audit_records = &tracker.audit_history;
let latest = audit_records.last().unwrap();

println!("Fees in period: {} µIPN", latest.fees_collected);
println!("Total lifetime fees: {} µIPN", latest.total_fees_collected);
```

---

## 8. Memory Monitoring

### Reading Actual Memory Usage
```rust
use ippan_ai_core::health::HealthMonitor;

let monitor = HealthMonitor::new(HealthConfig::default());

// Get real memory usage
let health = monitor.run_health_checks().await;
let memory_bytes = health.metadata
    .get("memory_usage_bytes")
    .and_then(|s| s.parse::<u64>().ok())
    .unwrap_or(0);

println!("Memory: {} MB", memory_bytes / 1_048_576);
```

### Platform-Specific Behavior
```
Linux:    Reads /proc/self/status (VmRSS)
Other:    Uses sysinfo crate
Fallback: Returns 100 MB if both fail
```

---

## Common Patterns

### Deterministic Message Construction
All signatures follow this pattern:
```rust
let mut message = Vec::new();
message.extend_from_slice(b"OPERATION_TYPE_PREFIX");
message.extend_from_slice(&field1);
message.extend_from_slice(&field2);
// ... add all relevant fields in fixed order

let message_hash = Sha256::digest(&message);
let signature = signing_key.sign(&message_hash);
```

### Error Handling
```rust
// All implementations return Result types
match operation() {
    Ok(value) => { /* success */ },
    Err(e) => {
        // Specific error types per crate
        // - NetworkError
        // - HandleRegistryError
        // - CryptoError
        // - AnchorError
    }
}
```

### Testing Your Code
```rust
#[test]
fn test_my_feature() {
    use ed25519_dalek::SigningKey;
    
    // Use deterministic keys for tests
    let key = SigningKey::from_bytes(&[42u8; 32]);
    
    // Create and sign
    let mut message = create_message();
    message.sign(&key.to_bytes()).unwrap();
    
    // Verify
    assert!(message.verify_signature(&key.verifying_key().to_bytes()));
}
```

---

## Migration Guide

### For Existing Network Nodes
1. Update to latest version
2. All nodes must upgrade together (breaking change)
3. Old unsigned messages will be rejected
4. No data migration needed

### For Handle Owners
1. Re-sign all existing handle operations
2. Use provided migration script (TBD)
3. Old handles remain valid during grace period
4. New operations require signatures

### For AI Model Providers
1. Existing registrations grandfathered (no fee)
2. New registrations subject to fees
3. Budget for registration costs
4. Larger models = higher fees

---

## Troubleshooting

### Signature Verification Fails
- Check key length (must be exactly 32 bytes)
- Verify message construction order matches
- Ensure using SHA-256 hash of message (not raw message)
- Check signature length (must be exactly 64 bytes)

### Memory Monitoring Returns Fallback
- On Linux: Check /proc/self/status is readable
- Ensure sysinfo crate installed
- Check process permissions
- Fallback (100MB) is safe but not accurate

### Fee Calculation Unexpected
- Check model size_bytes is set correctly
- Verify fee parameters (base + per_mb)
- Remember: size rounds UP to nearest MB
- Fee is in micro-IPN (µIPN), divide by 1M for IPN

---

## Best Practices

### Security
✅ Always verify signatures before state changes
✅ Use deterministic message construction
✅ Never reuse signatures across operations
✅ Validate all inputs before processing

### Performance
✅ Cache public keys when possible
✅ Batch signature verifications if supported
✅ Monitor signature operation latency
✅ Set alerts for >100µs average

### Testing
✅ Use deterministic keys in tests ([42u8; 32])
✅ Test both valid and invalid signatures
✅ Test edge cases (empty, max size, etc.)
✅ Verify deterministic behavior

---

## Support & Resources

- **Full Documentation**: PLACEHOLDER_FIXES_FINAL_REPORT.md
- **Initial Analysis**: PLACEHOLDER_IMPLEMENTATIONS_SUMMARY.md
- **Progress Tracking**: IMPLEMENTATION_PROGRESS.md

For questions or issues, contact the development team.

---

**Last Updated**: 2025-11-04  
**Version**: 1.0  
**Status**: Production Ready ✅
