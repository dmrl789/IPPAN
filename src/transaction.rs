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
    pub fn message(&self) -> String {
        format!("{}{}{}", self.from, self.to, self.amount)
    }

    pub fn verify(&self, verifying_key: &VerifyingKey) -> bool {
        if self.signature.len() != 64 {
            return false;
        }

        let sig_bytes: [u8; 64] = match self.signature.as_slice().try_into() {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };

        let message = self.message();

        // In ed25519-dalek 2.1.1, from_bytes returns Signature directly (not Result)
        let signature = Signature::from_bytes(&sig_bytes);

        verifying_key.verify(message.as_bytes(), &signature).is_ok()
    }
}
