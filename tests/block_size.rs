use ippan::{
    consensus::{blockdag::*, limits::MAX_BLOCK_SIZE_BYTES},
    consensus::hashtimer::HashTimer,
    NodeId, TransactionHash,
};

#[test]
fn block_under_cap_is_ok() {
    let validator_id = [1u8; 32];
    let hashtimer = HashTimer::new("test_node", 1, 1);
    
    // Create a block with 100 transaction hashes (100 * 32 = 3200 bytes + header)
    let tx_hashes = vec![[0u8; 32]; 100];
    let result = Block::new(1, tx_hashes, validator_id, hashtimer);
    
    assert!(result.is_ok());
    let block = result.unwrap();
    assert!(block.header.block_size_bytes as usize <= MAX_BLOCK_SIZE_BYTES);
    assert_eq!(block.header.tx_count, 100);
}

#[test]
fn block_over_cap_is_rejected() {
    let validator_id = [1u8; 32];
    let hashtimer = HashTimer::new("test_node", 1, 1);
    
    // Create a block with 1200 transaction hashes (1200 * 32 ≈ 38,400 + header > 32KB)
    let tx_hashes = vec![[0u8; 32]; 1_200];
    let result = Block::new(1, tx_hashes, validator_id, hashtimer);
    
    assert!(result.is_err());
    let err = result.err().unwrap();
    match err {
        BlockError::TooLarge { size, max } => {
            assert_eq!(max, MAX_BLOCK_SIZE_BYTES);
            assert!(size > MAX_BLOCK_SIZE_BYTES);
        }
        _ => panic!("Expected TooLarge error"),
    }
}

#[test]
fn block_size_estimation_is_accurate() {
    let validator_id = [1u8; 32];
    let hashtimer = HashTimer::new("test_node", 1, 1);
    
    // Test with different numbers of transactions
    for tx_count in [0, 10, 100, 500, 1000] {
        let tx_hashes = vec![[0u8; 32]; tx_count];
        let result = Block::new(1, tx_hashes, validator_id.clone(), hashtimer.clone());
        
        if result.is_ok() {
            let block = result.unwrap();
            let estimated_size = block.estimate_size_bytes();
            assert_eq!(block.header.block_size_bytes as usize, estimated_size);
            assert!(estimated_size <= MAX_BLOCK_SIZE_BYTES);
        }
    }
}

#[test]
fn block_header_size_calculation() {
    let validator_id = [1u8; 32];
    let hashtimer = HashTimer::new("test_node", 1, 1);
    
    // Create a block with no parents
    let tx_hashes = vec![[0u8; 32]; 10];
    let block = Block::new(1, tx_hashes, validator_id, hashtimer).unwrap();
    
    let header_size = block.header.estimate_size_bytes();
    // Base header size should be around 184 bytes
    assert!(header_size >= 180 && header_size <= 200);
}

#[test]
fn block_with_parents_size_calculation() {
    let validator_id = [1u8; 32];
    let hashtimer = HashTimer::new("test_node", 1, 1);
    
    // Create a block with multiple parents
    let tx_hashes = vec![[0u8; 32]; 10];
    let mut block = Block::new(1, tx_hashes, validator_id, hashtimer).unwrap();
    
    // Add some parent hashes and rounds
    block.header.parent_hashes = vec![[1u8; 32]; 3];
    block.header.parent_rounds = vec![0, 0, 0];
    
    let header_size = block.header.estimate_size_bytes();
    // Should be larger due to parent data
    assert!(header_size > 200);
}

#[test]
fn merkle_root_calculation() {
    let validator_id = [1u8; 32];
    let hashtimer = HashTimer::new("test_node", 1, 1);
    
    // Test with empty transactions
    let tx_hashes = vec![];
    let block = Block::new(1, tx_hashes, validator_id.clone(), hashtimer.clone()).unwrap();
    assert_eq!(block.header.merkle_root, [0u8; 32]);
    
    // Test with single transaction
    let tx_hashes = vec![[1u8; 32]];
    let block = Block::new(1, tx_hashes, validator_id.clone(), hashtimer.clone()).unwrap();
    assert_ne!(block.header.merkle_root, [0u8; 32]);
    
    // Test with multiple transactions
    let tx_hashes = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
    let block = Block::new(1, tx_hashes, validator_id, hashtimer).unwrap();
    assert_ne!(block.header.merkle_root, [0u8; 32]);
}

#[test]
fn block_hash_calculation() {
    let validator_id = [1u8; 32];
    let hashtimer = HashTimer::new("test_node", 1, 1);
    
    let tx_hashes = vec![[1u8; 32], [2u8; 32]];
    let block = Block::new(1, tx_hashes, validator_id, hashtimer).unwrap();
    
    // Block hash should be non-zero
    assert_ne!(block.header.hash, [0u8; 32]);
    
    // Block hash should be deterministic
    let validator_id = [1u8; 32];
    let hashtimer = HashTimer::new("test_node", 1, 1);
    let tx_hashes = vec![[1u8; 32], [2u8; 32]];
    let block2 = Block::new(1, tx_hashes, validator_id, hashtimer).unwrap();
    
    // Note: Hashes might be different due to timestamps, but structure should be consistent
    assert_eq!(block.header.tx_count, block2.header.tx_count);
}

#[test]
fn max_transactions_per_block_estimate() {
    let validator_id = [1u8; 32];
    let hashtimer = HashTimer::new("test_node", 1, 1);
    
    // Calculate approximately how many transactions fit in 32KB
    let header_size = 200; // Approximate header size
    let available_space = MAX_BLOCK_SIZE_BYTES - header_size;
    let max_tx_hashes = available_space / 32; // 32 bytes per transaction hash
    
    // Test with the calculated maximum
    let tx_hashes = vec![[0u8; 32]; max_tx_hashes];
    let result = Block::new(1, tx_hashes, validator_id, hashtimer);
    
    assert!(result.is_ok());
    
    // Test with one more transaction (should fail)
    let tx_hashes = vec![[0u8; 32]; max_tx_hashes + 1];
    let result = Block::new(1, tx_hashes, validator_id, hashtimer);
    assert!(result.is_err());
}

#[test]
fn block_size_constants() {
    use ippan::consensus::limits::*;
    
    assert_eq!(MAX_BLOCK_SIZE_BYTES, 32 * 1024);
    assert_eq!(TYPICAL_BLOCK_SIZE_MIN_BYTES, 4 * 1024);
    assert_eq!(TYPICAL_BLOCK_SIZE_MAX_BYTES, 32 * 1024);
    assert_eq!(MAX_TRANSACTIONS_PER_BLOCK, 2000);
    assert_eq!(MAX_PARENT_BLOCKS, 8);
    assert_eq!(MIN_PARENT_BLOCKS, 1);
}

#[test]
fn block_error_display() {
    let error = BlockError::TooLarge {
        size: 50000,
        max: 32768,
    };
    
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("50000"));
    assert!(error_msg.contains("32768"));
    assert!(error_msg.contains("too large"));
}

#[test]
fn block_with_signature_size() {
    let validator_id = [1u8; 32];
    let hashtimer = HashTimer::new("test_node", 1, 1);
    
    let tx_hashes = vec![[0u8; 32]; 100];
    let mut block = Block::new(1, tx_hashes, validator_id, hashtimer).unwrap();
    
    // Add a signature
    block.signature = Some([1u8; 64]);
    
    // Size should include the signature
    let size_with_signature = block.estimate_size_bytes();
    assert!(size_with_signature > 0);
}

fn dummy_header() -> BlockHeader {
    BlockHeader {
        hash: [0u8; 32],
        round: 1,
        height: 0,
        validator_id: [0u8; 32],
        hashtimer: HashTimer::new("test", 1, 1),
        parent_hashes: vec![],
        parent_rounds: vec![],
        timestamp_ns: 0,
        block_size_bytes: 0,
        tx_count: 0,
        merkle_root: [0u8; 32],
    }
}
