//! Deterministic ordering utilities for BlockDAG vertices.
//!
//! Blocks are primarily ordered by their HashTimer timestamps. When two blocks
//! share the same timestamp, the HashTimer digest provides a stable secondary
//! ordering to ensure deterministic merges across all nodes.

use std::cmp::Ordering;

use crate::block::BlockHeader;

/// Compare two [`BlockHeader`] instances according to IPPAN's ordering rules.
pub fn order_blocks(a: &BlockHeader, b: &BlockHeader) -> Ordering {
    a.hash_timer
        .timestamp_us
        .cmp(&b.hash_timer.timestamp_us)
        .then_with(|| a.hash_timer.digest().cmp(&b.hash_timer.digest()))
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand_core::OsRng;

    use crate::block::Block;

    #[test]
    fn headers_ordered_by_timestamp_then_digest() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let parent_hashes = vec![[0u8; 32]];

        let block_a = Block::new(&signing_key, parent_hashes.clone(), vec![b"a".to_vec()]);
        let block_b = Block::new(&signing_key, parent_hashes, vec![b"b".to_vec()]);

        let ordering = order_blocks(&block_a.header, &block_b.header);
        if block_a.header.hash_timer.timestamp_us != block_b.header.hash_timer.timestamp_us {
            assert_eq!(
                ordering,
                block_a
                    .header
                    .hash_timer
                    .timestamp_us
                    .cmp(&block_b.header.hash_timer.timestamp_us)
            );
        } else {
            assert_eq!(
                ordering,
                block_a
                    .header
                    .hash_timer
                    .digest()
                    .cmp(&block_b.header.hash_timer.digest())
            );
        }
    }
}
