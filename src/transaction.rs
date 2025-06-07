use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub signature: Vec<u8>,
}

impl Transaction {
    pub fn verify(&self, verifying_key: &VerifyingKey) -> bool {
        // Signature must be 64 bytes for ed25519
        if self.signature.len() != 64 {
            return false;
        }

        // Convert Vec<u8> to [u8; 64]
        let sig_bytes: [u8; 64] = match self.signature.as_slice().try_into() {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };

        // Prepare the message
        let message = format!("{}{}{}", self.from, self.to, self.amount);

        // Construct signature directly (no Result!)
        let signature = Signature::from_bytes(&sig_bytes);

        verifying_key.verify(message.as_bytes(), &signature).is_ok()
    }
}
