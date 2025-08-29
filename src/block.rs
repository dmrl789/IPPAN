use crate::crypto::{self, Hash};
use crate::transaction::Transaction;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const TARGET_BLOCK_SIZE: usize = 16 * 1024; // 16 KB
pub const MAX_BLOCK_SIZE: usize = 32 * 1024;    // 32 KB
pub const MIN_BLOCK_SIZE: usize = 4 * 1024;     // 4 KB
pub const MAX_PARENT_REFS: usize = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub parent_refs: Vec<Hash>,      // Up to 4 parent block hashes
    pub round_id: u64,               // Round this block belongs to
    pub block_time_us: u64,          // IPPAN time when block was created
    pub builder_id: Hash,            // Hash of builder's public key
    pub tx_count: u32,               // Number of transactions in block
    pub merkle_root: Hash,           // Merkle root of transaction hashes
    pub hash_timer: Hash,            // HashTimer for block ordering
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Hash>,     // Transaction hashes only (not full transactions)
}

impl Block {
    pub fn new(
        parent_refs: Vec<Hash>,
        round_id: u64,
        block_time_us: u64,
        builder_id: Hash,
        transactions: Vec<Transaction>,
    ) -> Result<Self, Error> {
        // Validate parent refs
        if parent_refs.len() > MAX_PARENT_REFS {
            return Err(Error::Block(format!(
                "Too many parent references: {} (max: {})",
                parent_refs.len(),
                MAX_PARENT_REFS
            )));
        }

        // Extract transaction hashes
        let tx_hashes: Vec<Hash> = transactions
            .iter()
            .map(|tx| tx.compute_id())
            .collect::<Result<Vec<Hash>, Error>>()?;

        // Compute merkle root
        let merkle_root = Self::compute_merkle_root(&tx_hashes)?;

        // Generate hash timer for block ordering
        let block_id = Self::compute_block_id(&parent_refs, round_id, block_time_us, &merkle_root)?;
        let hash_timer = crypto::generate_hash_timer(block_time_us, &[0u8; 16], &block_id);

        let header = BlockHeader {
            parent_refs,
            round_id,
            block_time_us,
            builder_id,
            tx_count: tx_hashes.len() as u32,
            merkle_root,
            hash_timer,
        };

        Ok(Self {
            header,
            transactions: tx_hashes,
        })
    }

    pub fn compute_id(&self) -> Result<Hash, Error> {
        Self::compute_block_id(
            &self.header.parent_refs,
            self.header.round_id,
            self.header.block_time_us,
            &self.header.merkle_root,
        )
    }

    fn compute_block_id(
        parent_refs: &[Hash],
        round_id: u64,
        block_time_us: u64,
        merkle_root: &Hash,
    ) -> Result<Hash, Error> {
        let mut data = Vec::new();
        
        // Parent refs
        for parent in parent_refs {
            data.extend_from_slice(parent);
        }
        
        // Round ID
        data.extend_from_slice(&round_id.to_le_bytes());
        
        // Block time
        data.extend_from_slice(&block_time_us.to_le_bytes());
        
        // Merkle root
        data.extend_from_slice(merkle_root);
        
        Ok(crypto::hash(&data))
    }

    fn compute_merkle_root(tx_hashes: &[Hash]) -> Result<Hash, Error> {
        if tx_hashes.is_empty() {
            return Ok([0u8; 32]);
        }

        let mut current_level: Vec<Hash> = tx_hashes.to_vec();
        
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            
            for chunk in current_level.chunks(2) {
                let mut hasher = crypto::blake3_hash(chunk[0].as_ref());
                if chunk.len() > 1 {
                    hasher = crypto::blake3_hash(&[hasher, chunk[1]].concat());
                }
                next_level.push(hasher);
            }
            
            current_level = next_level;
        }
        
        Ok(current_level[0])
    }

    pub fn verify(&self) -> Result<bool, Error> {
        // Check parent refs count
        if self.header.parent_refs.len() > MAX_PARENT_REFS {
            return Err(Error::Block("Too many parent references".to_string()));
        }

        // Check transaction count matches
        if self.header.tx_count as usize != self.transactions.len() {
            return Err(Error::Block("Transaction count mismatch".to_string()));
        }

        // Verify merkle root
        let computed_root = Self::compute_merkle_root(&self.transactions)?;
        if computed_root != self.header.merkle_root {
            return Err(Error::Block("Invalid merkle root".to_string()));
        }

        // Check block size
        let serialized = self.serialize()?;
        if serialized.len() > MAX_BLOCK_SIZE {
            return Err(Error::Block(format!(
                "Block too large: {} bytes (max: {})",
                serialized.len(),
                MAX_BLOCK_SIZE
            )));
        }

        Ok(true)
    }

    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        bincode::serialize(self).map_err(|e| Error::Serialization(e.to_string()))
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, Error> {
        bincode::deserialize(data).map_err(|e| Error::Serialization(e.to_string()))
    }

    pub fn size(&self) -> Result<usize, Error> {
        Ok(self.serialize()?.len())
    }

    pub fn get_sort_key(&self) -> Result<(Hash, Hash), Error> {
        let block_id = self.compute_id()?;
        Ok((self.header.hash_timer, block_id))
    }

    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }
}

pub struct BlockBuilder {
    target_size: usize,
    max_size: usize,
    min_size: usize,
}

impl BlockBuilder {
    pub fn new() -> Self {
        Self {
            target_size: TARGET_BLOCK_SIZE,
            max_size: MAX_BLOCK_SIZE,
            min_size: MIN_BLOCK_SIZE,
        }
    }

    pub fn with_target_size(mut self, target_size: usize) -> Self {
        self.target_size = target_size;
        self
    }

    pub fn build_block(
        &self,
        parent_refs: Vec<Hash>,
        round_id: u64,
        block_time_us: u64,
        builder_id: Hash,
        transactions: Vec<Transaction>,
    ) -> Result<Block, Error> {
        // Check if we have enough transactions to meet minimum size
        let mut selected_txs = Vec::new();
        let mut current_size = 0;

        for tx in transactions {
            let tx_size = tx.size()?;
            
            // Check if adding this transaction would exceed max size
            if current_size + tx_size > self.max_size {
                break;
            }
            
            selected_txs.push(tx);
            current_size += tx_size;
            
            // Stop if we've reached target size
            if current_size >= self.target_size {
                break;
            }
        }

        // Create block
        Block::new(parent_refs, round_id, block_time_us, builder_id, selected_txs)
    }

    pub fn estimate_block_size(&self, transactions: &[Transaction]) -> Result<usize, Error> {
        let mut total_size = 0;
        
        // Estimate header size (fixed)
        total_size += 32 * MAX_PARENT_REFS; // parent_refs
        total_size += 8; // round_id
        total_size += 8; // block_time_us
        total_size += 32; // builder_id
        total_size += 4; // tx_count
        total_size += 32; // merkle_root
        total_size += 32; // hash_timer
        
        // Add transaction hash sizes
        total_size += transactions.len() * 32; // Each tx hash is 32 bytes
        
        Ok(total_size)
    }
}

impl Default for BlockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;
    use crate::time::IppanTime;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_block_creation() {
        let keypair = KeyPair::generate();
        let recipient = KeyPair::generate();
        let ippan_time = Arc::new(IppanTime::new());
        
        let tx = Transaction::new(
            &keypair,
            recipient.public_key,
            1000,
            1,
            ippan_time,
        ).unwrap();
        
        let block = Block::new(
            vec![],
            1,
            1234567890,
            [1u8; 32],
            vec![tx],
        ).unwrap();
        
        assert_eq!(block.header.tx_count, 1);
        assert_eq!(block.transactions.len(), 1);
    }

    #[tokio::test]
    async fn test_block_verification() {
        let keypair = KeyPair::generate();
        let recipient = KeyPair::generate();
        let ippan_time = Arc::new(IppanTime::new());
        
        let tx = Transaction::new(
            &keypair,
            recipient.public_key,
            1000,
            1,
            ippan_time,
        ).unwrap();
        
        let block = Block::new(
            vec![],
            1,
            1234567890,
            [1u8; 32],
            vec![tx],
        ).unwrap();
        
        assert!(block.verify().unwrap());
    }

    #[tokio::test]
    async fn test_block_serialization() {
        let keypair = KeyPair::generate();
        let recipient = KeyPair::generate();
        let ippan_time = Arc::new(IppanTime::new());
        
        let tx = Transaction::new(
            &keypair,
            recipient.public_key,
            1000,
            1,
            ippan_time,
        ).unwrap();
        
        let block = Block::new(
            vec![],
            1,
            1234567890,
            [1u8; 32],
            vec![tx],
        ).unwrap();
        
        let serialized = block.serialize().unwrap();
        let deserialized = Block::deserialize(&serialized).unwrap();
        
        assert_eq!(block.header.tx_count, deserialized.header.tx_count);
        assert_eq!(block.transactions.len(), deserialized.transactions.len());
    }

    #[tokio::test]
    async fn test_block_builder() {
        let builder = BlockBuilder::new();
        let keypair = KeyPair::generate();
        let recipient = KeyPair::generate();
        let ippan_time = Arc::new(IppanTime::new());
        
        let tx = Transaction::new(
            &keypair,
            recipient.public_key,
            1000,
            1,
            ippan_time,
        ).unwrap();
        
        let block = builder.build_block(
            vec![],
            1,
            1234567890,
            [1u8; 32],
            vec![tx],
        ).unwrap();
        
        assert_eq!(block.header.tx_count, 1);
    }

    #[tokio::test]
    async fn test_merkle_root_computation() {
        let hashes = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let root = Block::compute_merkle_root(&hashes).unwrap();
        assert_eq!(root.len(), 32);
    }
}
