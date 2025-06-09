use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub previous_hash: String,
    pub hash: String,
    pub timestamp: u64,
    // Add: pub transactions: Vec<Transaction> if you want domain actions in blocks!
}
