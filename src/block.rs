use serde::{Serialize, Deserialize};
use crate::transaction::Transaction;
use sha2::{Sha256, Digest};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub index: u64,
    pub previous_hash: String,
    pub transactions: Vec<Transaction>,
    pub reward: u64,
    pub miner: String,
    pub hash: String,
}

impl Block {
    /// Create a new block with hash
    pub fn new(
        index: u64,
        previous_hash: String,
        transactions: Vec<Transaction>,
        reward: u64,
        miner: String,
    ) -> Self {
        let hash = Self::calculate_hash(index, &previous_hash, &transactions, reward, &miner);
        Block {
            index,
            previous_hash,
            transactions,
            reward,
            miner,
            hash,
        }
    }

    /// Calculate hash for block data
    pub fn calculate_hash(
        index: u64,
        previous_hash: &str,
        transactions: &Vec<Transaction>,
        reward: u64,
        miner: &str,
    ) -> String {
        let data = format!(
            "{}{}{:?}{}{}",
            index,
            previous_hash,
            transactions,
            reward,
            miner
        );
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash_bytes = hasher.finalize();
        hex::encode(hash_bytes)
    }

    /// Validate block (hash, prev hash, index)
    pub fn is_valid(&self, previous: &Block) -> bool {
        // Check previous hash linkage
        if self.previous_hash != previous.hash {
            return false;
        }
        // Check sequential index
        if self.index != previous.index + 1 {
            return false;
        }
        // Check the hash is valid for this block's data
        let expected_hash = Self::calculate_hash(
            self.index,
            &self.previous_hash,
            &self.transactions,
            self.reward,
            &self.miner,
        );
        self.hash == expected_hash
    }
}
