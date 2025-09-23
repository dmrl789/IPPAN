use crate::hashtimer::{random_nonce, HashTimer, IppanTimeMicros};
use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};

/// Block header containing metadata and HashTimer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Previous block hash (32 bytes)
    pub prev_hash: [u8; 32],
    /// Merkle root of transactions (32 bytes)
    pub tx_merkle_root: [u8; 32],
    /// Block height/round number
    pub round_id: u64,
    /// Proposer node ID (32 bytes)
    pub proposer_id: [u8; 32],
    /// Nonce for proof-of-work or other consensus mechanism
    pub nonce: u64,
    /// HashTimer for temporal ordering and validation
    pub hashtimer: HashTimer,
    /// Timestamp when block was created
    pub timestamp: IppanTimeMicros,
}

/// A block in the IPPAN blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Block header
    pub header: BlockHeader,
    /// List of transactions in this block
    pub transactions: Vec<Transaction>,
}

impl Block {
    /// Create a new block
    pub fn new(
        prev_hash: [u8; 32],
        transactions: Vec<Transaction>,
        round_id: u64,
        proposer_id: [u8; 32],
    ) -> Self {
        let timestamp = IppanTimeMicros::now();
        let nonce_bytes = random_nonce();
        let nonce = u64::from_be_bytes([
            nonce_bytes[0],
            nonce_bytes[1],
            nonce_bytes[2],
            nonce_bytes[3],
            nonce_bytes[4],
            nonce_bytes[5],
            nonce_bytes[6],
            nonce_bytes[7],
        ]);

        // Compute transaction merkle root
        let tx_merkle_root = Self::compute_merkle_root(&transactions);

        // Create HashTimer for this block
        let payload =
            Self::create_payload(&prev_hash, &tx_merkle_root, round_id, &proposer_id, nonce);
        let hashtimer = HashTimer::now_block("block", &payload, &nonce_bytes, &proposer_id);

        let header = BlockHeader {
            prev_hash,
            tx_merkle_root,
            round_id,
            proposer_id,
            nonce: nonce,
            hashtimer,
            timestamp,
        };

        Self {
            header,
            transactions,
        }
    }

    /// Create payload for HashTimer computation
    fn create_payload(
        prev_hash: &[u8; 32],
        tx_merkle_root: &[u8; 32],
        round_id: u64,
        proposer_id: &[u8; 32],
        nonce: u64,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(prev_hash);
        payload.extend_from_slice(tx_merkle_root);
        payload.extend_from_slice(&round_id.to_be_bytes());
        payload.extend_from_slice(proposer_id);
        payload.extend_from_slice(&nonce.to_be_bytes());
        payload
    }

    /// Compute merkle root of transactions
    fn compute_merkle_root(transactions: &[Transaction]) -> [u8; 32] {
        if transactions.is_empty() {
            // Empty block has zero merkle root
            return [0u8; 32];
        }

        if transactions.len() == 1 {
            // Single transaction, its hash is the merkle root
            return transactions[0].hash();
        }

        // Compute merkle tree
        let mut hashes: Vec<[u8; 32]> = transactions.iter().map(|tx| tx.hash()).collect();

        while hashes.len() > 1 {
            let mut next_level = Vec::new();

            for i in (0..hashes.len()).step_by(2) {
                let left = hashes[i];
                let right = if i + 1 < hashes.len() {
                    hashes[i + 1]
                } else {
                    left // Duplicate last element if odd number
                };

                let mut hasher = blake3::Hasher::new();
                hasher.update(&left);
                hasher.update(&right);
                let hash = hasher.finalize();

                let mut combined = [0u8; 32];
                combined.copy_from_slice(&hash.as_bytes()[0..32]);
                next_level.push(combined);
            }

            hashes = next_level;
        }

        hashes[0]
    }

    /// Get block hash
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.header.prev_hash);
        hasher.update(&self.header.tx_merkle_root);
        hasher.update(&self.header.round_id.to_be_bytes());
        hasher.update(&self.header.proposer_id);
        hasher.update(&self.header.nonce.to_be_bytes());
        hasher.update(&self.header.hashtimer.to_hex().as_bytes());
        hasher.update(&self.header.timestamp.0.to_be_bytes());

        let hash = hasher.finalize();
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash.as_bytes()[0..32]);
        result
    }

    /// Verify block validity
    pub fn is_valid(&self) -> bool {
        // Verify merkle root matches transactions
        let computed_merkle_root = Self::compute_merkle_root(&self.transactions);
        if computed_merkle_root != self.header.tx_merkle_root {
            return false;
        }

        // Verify all transactions are valid
        for tx in &self.transactions {
            if !tx.is_valid() {
                return false;
            }
        }

        // Verify HashTimer is valid
        self.header.hashtimer.time().0 <= IppanTimeMicros::now().0

        // Note: In a real implementation, we might want to verify the exact HashTimer
        // but for now we just check that the time is reasonable
    }

    /// Get block size in bytes (approximate)
    pub fn size(&self) -> usize {
        // Rough estimate of block size
        let header_size = 32 + 32 + 8 + 32 + 8 + 64 + 8; // All header fields
        let tx_size = self.transactions.len() * 200; // Rough estimate per transaction
        header_size + tx_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_creation() {
        let prev_hash = [1u8; 32];
        let proposer_id = [2u8; 32];
        let round_id = 1;
        let transactions = vec![
            Transaction::new([3u8; 32], [4u8; 32], 1000, 1),
            Transaction::new([5u8; 32], [6u8; 32], 2000, 2),
        ];

        let block = Block::new(prev_hash, transactions, round_id, proposer_id);

        assert_eq!(block.header.prev_hash, prev_hash);
        assert_eq!(block.header.proposer_id, proposer_id);
        assert_eq!(block.header.round_id, round_id);
        assert_eq!(block.transactions.len(), 2);
        assert!(block.header.hashtimer.to_hex().len() == 64);
    }

    #[test]
    fn test_empty_block() {
        let prev_hash = [1u8; 32];
        let proposer_id = [2u8; 32];
        let round_id = 1;
        let transactions = vec![];

        let block = Block::new(prev_hash, transactions, round_id, proposer_id);

        assert_eq!(block.transactions.len(), 0);
        assert_eq!(block.header.tx_merkle_root, [0u8; 32]);
    }

    #[test]
    fn test_single_transaction_block() {
        let prev_hash = [1u8; 32];
        let proposer_id = [2u8; 32];
        let round_id = 1;
        let tx = Transaction::new([3u8; 32], [4u8; 32], 1000, 1);
        let transactions = vec![tx.clone()];

        let block = Block::new(prev_hash, transactions, round_id, proposer_id);

        assert_eq!(block.transactions.len(), 1);
        assert_eq!(block.header.tx_merkle_root, tx.hash());
    }

    #[test]
    fn test_merkle_root_computation() {
        let tx1 = Transaction::new([1u8; 32], [2u8; 32], 1000, 1);
        let tx2 = Transaction::new([3u8; 32], [4u8; 32], 2000, 2);
        let transactions = vec![tx1, tx2];

        let merkle_root = Block::compute_merkle_root(&transactions);
        assert_ne!(merkle_root, [0u8; 32]);
    }

    #[test]
    fn test_block_hash() {
        let prev_hash = [1u8; 32];
        let proposer_id = [2u8; 32];
        let round_id = 1;
        let transactions = vec![Transaction::new([3u8; 32], [4u8; 32], 1000, 1)];

        let block = Block::new(prev_hash, transactions, round_id, proposer_id);
        let hash = block.hash();

        assert_ne!(hash, [0u8; 32]);
    }
}
