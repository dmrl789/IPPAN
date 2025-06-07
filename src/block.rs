use crate::transaction::Transaction;
use serde::{Serialize, Deserialize};
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
        let mut block = Block {
            index,
            previous_hash,
            timestamp,
            transactions,
            nonce,
            hash: String::new(),
            reward,
        };
        let hash = block.calculate_hash();
        block.hash = hash;
        block
    }

    pub fn calculate_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let serialized = bincode::serialize(self).expect("serialize for hash");
        let mut hasher = Sha256::new();
        hasher.update(serialized);
        hex::encode(hasher.finalize())
    }

    // --- Binary Serialization ---
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Block binary serialization failed")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }

    // --- JSON Serialization (optional, for debugging) ---
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}
