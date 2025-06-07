use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub signature: Vec<u8>, // Signature as bytes
}

impl Transaction {
    pub fn new(sender: String, recipient: String, amount: u64, signature: Vec<u8>) -> Self {
        Self { sender, recipient, amount, signature }
    }

    /// Just a placeholder for a real signature check!
    pub fn is_valid(&self, _verifying_key: &VerifyingKey) -> bool {
        // TODO: Implement actual signature verification
        true
    }
}
