use crate::transaction::Transaction;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub previous_hash: String,
    pub hash: String,
    pub author: String,
    pub reward: u64,
    pub transactions: Vec<Transaction>, // Add this!
}

impl Block {
    pub fn new(transactions: Vec<Transaction>, previous_hash: String, author: String, reward: u64) -> Self {
        let index = 0; // (set appropriately if needed)
        let timestamp = chrono::Utc::now().timestamp() as u64;
        let hash = format!("{:x}", blake3::hash(format!("{:?}{:?}{:?}", &transactions, &previous_hash, &author).as_bytes()));
        Self {
            index,
            timestamp,
            previous_hash,
            hash,
            author,
            reward,
            transactions,
        }
    }

    pub fn genesis() -> Self {
        Block::new(vec![], "0".repeat(64), "genesis".to_string(), 0)
    }
}
