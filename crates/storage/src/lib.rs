use anyhow::Result;
use ippan_types::{Block, Transaction};
use std::collections::HashMap;

/// Storage interface for IPPAN blockchain
pub trait Storage {
    /// Store a block
    fn store_block(&mut self, block: Block) -> Result<()>;
    
    /// Get a block by hash
    fn get_block(&self, hash: &[u8; 32]) -> Result<Option<Block>>;
    
    /// Get a block by height
    fn get_block_by_height(&self, height: u64) -> Result<Option<Block>>;
    
    /// Store a transaction
    fn store_transaction(&mut self, tx: Transaction) -> Result<()>;
    
    /// Get a transaction by hash
    fn get_transaction(&self, hash: &[u8; 32]) -> Result<Option<Transaction>>;
    
    /// Get the latest block height
    fn get_latest_height(&self) -> Result<u64>;
}

/// In-memory storage implementation (for testing/development)
pub struct MemoryStorage {
    blocks: HashMap<String, Block>,
    transactions: HashMap<String, Transaction>,
    latest_height: u64,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            transactions: HashMap::new(),
            latest_height: 0,
        }
    }
}

impl Storage for MemoryStorage {
    fn store_block(&mut self, block: Block) -> Result<()> {
        let hash = block.hash();
        let hash_str = hex::encode(hash);
        
        self.blocks.insert(hash_str, block);
        self.latest_height = self.latest_height.max(self.blocks.len() as u64 - 1);
        
        Ok(())
    }
    
    fn get_block(&self, hash: &[u8; 32]) -> Result<Option<Block>> {
        let hash_str = hex::encode(hash);
        Ok(self.blocks.get(&hash_str).cloned())
    }
    
    fn get_block_by_height(&self, height: u64) -> Result<Option<Block>> {
        // In a real implementation, we'd have a height index
        Ok(self.blocks.values().find(|b| b.header.round_id == height).cloned())
    }
    
    fn store_transaction(&mut self, tx: Transaction) -> Result<()> {
        let hash = tx.hash();
        let hash_str = hex::encode(hash);
        
        self.transactions.insert(hash_str, tx);
        Ok(())
    }
    
    fn get_transaction(&self, hash: &[u8; 32]) -> Result<Option<Transaction>> {
        let hash_str = hex::encode(hash);
        Ok(self.transactions.get(&hash_str).cloned())
    }
    
    fn get_latest_height(&self) -> Result<u64> {
        Ok(self.latest_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_types::{Block, Transaction};

    #[test]
    fn test_memory_storage() {
        let mut storage = MemoryStorage::new();
        
        // Test storing and retrieving a block
        let block = Block::new([1u8; 32], vec![], 1, [2u8; 32]);
        let block_hash = block.hash();
        
        storage.store_block(block.clone()).unwrap();
        let retrieved_block = storage.get_block(&block_hash).unwrap();
        
        assert!(retrieved_block.is_some());
        assert_eq!(retrieved_block.unwrap().header.round_id, 1);
    }
}
