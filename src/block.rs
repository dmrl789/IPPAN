use serde::{Serialize, Deserialize};
use crate::transaction::Transaction;
use chrono::Utc;
use sha2::{Sha256, Digest};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    pub index: u64,
    pub previous_hash: String,
    pub timestamp: i64,
    pub transactions: Vec<Transaction>,
    pub nonce: u64,
    pub hash: String,
    pub reward: u64,
}

impl Block {
    pub fn new(index: u64, previous_hash: String, transactions: Vec<Transaction>, nonce: u64, reward: u64) -> Self {
        let timestamp = Utc::now().timestamp();
        let mut hasher = Sha256::new();
        hasher.update(format!("{}{}{}{:?}{}{}", index, &previous_hash, timestamp, &transactions, nonce, reward));
        let hash = format!("{:x}", hasher.finalize());

        Block {
            index,
            previous_hash,
            timestamp,
            transactions,
            nonce,
            hash,
            reward,
        }
    }
}
