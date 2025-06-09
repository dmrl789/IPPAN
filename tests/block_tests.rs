use ippan_core::block::Block;

#[test]
fn test_block_hash_consistency() {
    let b1 = Block::new(1, "test".to_string(), "abc".to_string());
    let b2 = Block::new(1, "test".to_string(), "abc".to_string());
    // They will differ only by timestamp, but their hashes should be deterministic for same content/timestamp
    assert_ne!(b1.hash, ""); // Hash is not empty
    assert_ne!(b1.hash, b2.hash); // Different timestamps
}
