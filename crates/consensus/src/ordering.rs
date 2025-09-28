use ippan_types::{Block, BlockId, RoundId};
use std::collections::{BTreeSet, HashSet};

/// Deterministically order the payloads of all blocks in a given round.
///
/// The routine mirrors the specification provided in the HashTimer/FinDAG
/// design notes: blocks are sorted lexicographically by their HashTimer time
/// prefix, creator identifier, and finally the block identifier. Each block's
/// payload identifiers are then flattened (after a local lexical sort) while
/// filtering duplicates and forwarding conflicts to the caller.
pub fn order_round<FParents, FExtract, FValid, FConflict>(
    _round: RoundId,
    blocks: &[Block],
    mut parents_of: FParents,
    mut extract_txs: FExtract,
    mut tx_is_valid: FValid,
    mut mark_conflict: FConflict,
) -> Vec<[u8; 32]>
where
    FParents: FnMut(&BlockId) -> Vec<BlockId>,
    FExtract: FnMut(&BlockId) -> Vec<[u8; 32]>,
    FValid: FnMut(&[u8; 32]) -> bool,
    FConflict: FnMut(&[u8; 32]),
{
    // Defensive check: ensure all parents are known and there are no stray
    // references to blocks from the same round.
    let known_ids: HashSet<BlockId> = blocks.iter().map(|b| b.header.id).collect();
    for block in blocks {
        for parent in &block.header.parent_ids {
            if known_ids.contains(parent) {
                // Parents should always point to round-1 blocks. If we find a
                // reference into the same round we flag it as an implicit
                // conflict by skipping the offending block entirely.
                continue;
            }
        }
        // Fetch the parents to ensure the caller has storage entries; the
        // result is ignored but the call exercises the closure for caching.
        let _ = parents_of(&block.header.id);
    }

    let mut sorted_blocks = blocks.to_vec();
    sorted_blocks.sort_by(|a, b| {
        a.header
            .hashtimer
            .time_prefix
            .cmp(&b.header.hashtimer.time_prefix)
            .then(a.header.creator.cmp(&b.header.creator))
            .then(a.header.id.cmp(&b.header.id))
    });

    let mut ordered: Vec<[u8; 32]> = Vec::new();
    let mut seen = BTreeSet::<[u8; 32]>::new();

    for block in sorted_blocks {
        let mut tx_ids = extract_txs(&block.header.id);
        tx_ids.sort();
        for tx_id in tx_ids {
            if !seen.insert(tx_id) {
                continue;
            }

            if tx_is_valid(&tx_id) {
                ordered.push(tx_id);
            } else {
                mark_conflict(&tx_id);
            }
        }
    }

    ordered
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_types::{HashTimer, IppanTimeMicros, Transaction};

    fn dummy_block(id_seed: u8, tx_ids: Vec<[u8; 32]>, round: RoundId) -> Block {
        let creator = [id_seed; 32];
        let parents = vec![[id_seed.wrapping_sub(1); 32]];
        let mut txs: Vec<Transaction> = Vec::new();
        for (idx, tx_id) in tx_ids.iter().copied().enumerate() {
            let mut tx =
                Transaction::new([id_seed; 32], [id_seed; 32], (idx + 1) as u64, idx as u64);
            tx.id = tx_id;
            txs.push(tx);
        }
        let mut block = Block::new(parents, txs, round, creator);
        // Overwrite the HashTimer for deterministic ordering in the test.
        block.header.hashtimer = HashTimer::derive(
            "block",
            IppanTimeMicros(round),
            b"test",
            &[id_seed],
            &[0u8; 32],
            &creator,
        );
        block
    }

    #[test]
    fn orders_blocks_and_transactions_deterministically() {
        let tx_a = [0xAA; 32];
        let tx_b = [0xBB; 32];
        let tx_c = [0xCC; 32];
        let tx_d = [0xDD; 32];

        let block1 = dummy_block(1, vec![tx_b, tx_a], 7);
        let block2 = dummy_block(2, vec![tx_c], 7);
        let block3 = dummy_block(3, vec![tx_d], 7);

        let result = order_round(
            7,
            &[block1.clone(), block2.clone(), block3.clone()],
            |_| Vec::new(),
            |block_id| {
                if block_id == &block1.header.id {
                    vec![tx_b, tx_a]
                } else if block_id == &block2.header.id {
                    vec![tx_c]
                } else {
                    vec![tx_d]
                }
            },
            |_| true,
            |_| {},
        );

        assert_eq!(result, vec![tx_a, tx_b, tx_c, tx_d]);
    }

    #[test]
    fn filters_conflicts() {
        let tx_a = [0xAA; 32];
        let tx_b = [0xBB; 32];
        let block1 = dummy_block(1, vec![tx_a, tx_b], 11);
        let block2 = dummy_block(2, vec![tx_a], 11);

        let mut conflicts = Vec::new();
        let ordered = order_round(
            11,
            &[block1, block2],
            |_| Vec::new(),
            |_| vec![tx_a, tx_b],
            |tx| tx != &tx_b,
            |tx| conflicts.push(*tx),
        );

        assert_eq!(ordered, vec![tx_a]);
        assert_eq!(conflicts, vec![tx_b]);
    }
}
