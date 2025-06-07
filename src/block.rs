use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest};

#[derive(Debug, Clone)]
pub struct Block {
    pub data: String,
    pub previous_hash: String,
    pub timestamp: u64,
    pub author: String,
    pub hash: String,
    pub reward: u64, // New: block reward field
}

impl Block {
    pub fn new(data: String, previous_hash: String, author: String, reward: u64) -> Self {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let hash = Block::calculate_hash(&data, &previous_hash, timestamp, &author, reward);
        Block {
            data,
            previous_hash,
            timestamp,
            author,
            hash,
            reward,
        }
    }

    pub fn calculate_hash(
        data: &str,
        previous_hash: &str,
        timestamp: u64,
        author: &str,
        reward: u64,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hasher.update(previous_hash.as_bytes());
        hasher.update(timestamp.to_le_bytes());
        hasher.update(author.as_bytes());
        hasher.update(reward.to_le_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    pub fn genesis() -> Self {
        Block::new(
            "genesis".to_string(),
            "0".repeat(64),
            "network".to_string(),
            0, // Genesis block: reward 0
        )
    }
}
