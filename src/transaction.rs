use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub signature: Vec<u8>, // 64 bytes when present
}

impl Transaction {
    /// Deterministic representation of the transaction for signing
    pub fn message(&self) -> Vec<u8> {
        // Consistent serialization for signing (can be replaced with more robust serialization)
        format!("{}{}{}", self.from, self.to, self.amount).as_bytes().to_vec()
    }

    /// Signature verification: checks that the signature matches the message and public key
    pub fn verify(&self, verifying_key: &VerifyingKey) -> bool {
        if self.signature.len() != 64 {
            return false;
        }
        let sig_bytes: [u8; 64] = match self.signature.as_slice().try_into() {
            Ok(arr) => arr,
            Err(_) => return false,
        };
        let signature = match Signature::from_bytes(&sig_bytes) {
            Ok(sig) => sig,
            Err(_) => return false,
        };
        verifying_key.verify(&self.message(), &signature).is_ok()
    }
}
