use crate::transaction::Transaction;
use serde::{Serialize, Deserialize};
use chrono::Utc;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub previous_hash: String,
    pub timestamp: i64,
    pub transactions: Vec<Transaction>,
    pub reward: u64,
    pub miner: String,
    pub hash: String,
    pub nonce: u64,
}

impl Block {
    pub fn genesis() -> Self {
        Block {
            index: 0,
            previous_hash: "0".repeat(64),
            timestamp: Utc::now().timestamp(),
            transactions: vec![],
            reward: 0,
            miner: "genesis".to_string(),
            hash: "GENESIS".to_string(),
            nonce: 0,
        }
    }

    pub fn new(
        index: u64,
        previous_hash: String,
        transactions: Vec<Transaction>,
        reward: u64,
        miner: String,
    ) -> Self {
        let timestamp = Utc::now().timestamp();
        let nonce = 0; // Add real PoW or randomness if needed
        let hash = Self::calculate_hash(index, &previous_hash, timestamp, &transactions, reward, &miner, nonce);
        Block {
            index,
            previous_hash,
            timestamp,
            transactions,
            reward,
            miner,
            hash,
            nonce,
        }
    }

    pub fn calculate_hash(
        index: u64,
        previous_hash: &str,
        timestamp: i64,
        transactions: &Vec<Transaction>,
        reward: u64,
        miner: &str,
        nonce: u64,
    ) -> String {
        let tx_bytes = bincode::serialize(&transactions).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(index.to_be_bytes());
        hasher.update(previous_hash.as_bytes());
        hasher.update(timestamp.to_be_bytes());
        hasher.update(&tx_bytes);
        hasher.update(reward.to_be_bytes());
        hasher.update(miner.as_bytes());
        hasher.update(nonce.to_be_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }
}
