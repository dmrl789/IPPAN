use crate::hashtimer::{HashTimer, IppanTimeMicros};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_bytes;

/// A transaction in the IPPAN blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction ID (32 bytes)
    pub id: [u8; 32],
    /// Sender address (32 bytes)
    pub from: [u8; 32],
    /// Recipient address (32 bytes)
    pub to: [u8; 32],
    /// Amount to transfer
    pub amount: u64,
    /// Nonce for replay protection
    pub nonce: u64,
    /// Transaction signature (64 bytes)
    #[serde(with = "serde_bytes")]
    pub signature: [u8; 64],
    /// HashTimer for temporal ordering and validation
    pub hashtimer: HashTimer,
    /// Timestamp when transaction was created
    pub timestamp: IppanTimeMicros,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(from: [u8; 32], to: [u8; 32], amount: u64, nonce: u64) -> Self {
        let timestamp = IppanTimeMicros::now();
        let payload = Self::create_payload(&from, &to, amount, nonce);
        let hashtimer = HashTimer::derive(
            "transaction",
            timestamp,
            b"transaction",
            &payload,
            &nonce.to_be_bytes(),
            &from,
        );

        Self {
            id: [0u8; 32], // Will be computed after signing
            from,
            to,
            amount,
            nonce,
            signature: [0u8; 64], // Will be set after signing
            hashtimer,
            timestamp,
        }
    }

    /// Create payload for HashTimer computation
    fn create_payload(from: &[u8; 32], to: &[u8; 32], amount: u64, nonce: u64) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(from);
        payload.extend_from_slice(to);
        payload.extend_from_slice(&amount.to_be_bytes());
        payload.extend_from_slice(&nonce.to_be_bytes());
        payload
    }

    /// Bytes used for signature verification (excludes signature and id)
    fn message_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(
            self.from.len() + self.to.len() + 8 + 8 + self.signature.len() + 7 + 25 + 8,
        );
        bytes.extend_from_slice(&self.from);
        bytes.extend_from_slice(&self.to);
        bytes.extend_from_slice(&self.amount.to_be_bytes());
        bytes.extend_from_slice(&self.nonce.to_be_bytes());
        bytes.extend_from_slice(&self.hashtimer.time_prefix);
        bytes.extend_from_slice(&self.hashtimer.hash_suffix);
        bytes.extend_from_slice(&self.timestamp.0.to_be_bytes());
        bytes
    }

    /// Bytes used for hashing (includes signature)
    fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = self.message_bytes();
        bytes.extend_from_slice(&self.signature);
        bytes
    }

    /// Sign the transaction using an Ed25519 private key
    pub fn sign(&mut self, private_key: &[u8; 32]) -> Result<(), String> {
        let signing_key = SigningKey::try_from(private_key.as_slice())
            .map_err(|e| format!("invalid private key: {e}"))?;
        let expected_public_key = signing_key.verifying_key().to_bytes();

        if self.from != expected_public_key {
            return Err("private key does not match transaction sender".into());
        }

        let message = self.message_bytes();
        let signature = signing_key.sign(&message);
        self.signature.copy_from_slice(&signature.to_bytes());
        self.id = self.hash();

        Ok(())
    }

    /// Verify the transaction signature
    pub fn verify(&self) -> bool {
        match VerifyingKey::from_bytes(&self.from) {
            Ok(verifying_key) => {
                let signature = Signature::from_bytes(&self.signature);
                verifying_key
                    .verify(&self.message_bytes(), &signature)
                    .is_ok()
            }
            Err(_) => false,
        }
    }

    /// Get transaction hash
    pub fn hash(&self) -> [u8; 32] {
        let hash = blake3::hash(&self.canonical_bytes());
        let mut result = [0u8; 32];
        result.copy_from_slice(hash.as_bytes());
        result
    }

    /// Check if transaction is valid
    pub fn is_valid(&self) -> bool {
        // Basic validation checks
        if self.amount == 0 {
            return false;
        }

        if self.from == self.to {
            return false;
        }

        // Verify signature
        if !self.verify() {
            return false;
        }

        if self.id != self.hash() {
            return false;
        }

        // Verify HashTimer is valid
        let payload = Self::create_payload(&self.from, &self.to, self.amount, self.nonce);
        let expected_hashtimer = HashTimer::derive(
            "transaction",
            self.timestamp,
            b"transaction",
            &payload,
            &self.nonce.to_be_bytes(),
            &self.from,
        );

        if expected_hashtimer != self.hashtimer {
            return false;
        }

        // HashTimer should be consistent (allowing for time differences)
        self.hashtimer.time().0 <= IppanTimeMicros::now().0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand_core::{OsRng, RngCore};

    fn generate_account() -> ([u8; 32], [u8; 32]) {
        let mut rng = OsRng;
        let mut secret = [0u8; 32];
        rng.fill_bytes(&mut secret);
        let signing_key = SigningKey::try_from(secret.as_slice()).unwrap();
        let public_key = signing_key.verifying_key().to_bytes();
        (secret, public_key)
    }

    #[test]
    fn test_transaction_creation() {
        let (_, from) = generate_account();
        let (_, to) = generate_account();
        let amount = 1000;
        let nonce = 1;

        let tx = Transaction::new(from, to, amount, nonce);

        assert_eq!(tx.from, from);
        assert_eq!(tx.to, to);
        assert_eq!(tx.amount, amount);
        assert_eq!(tx.nonce, nonce);
        assert!(tx.hashtimer.to_hex().len() == 64);
    }

    #[test]
    fn test_transaction_signing() {
        let (private_key, from) = generate_account();
        let (_, to) = generate_account();
        let mut tx = Transaction::new(from, to, 1000, 1);

        let result = tx.sign(&private_key);
        assert!(result.is_ok());
        assert_ne!(tx.signature, [0u8; 64]);
        assert_ne!(tx.id, [0u8; 32]);
    }

    #[test]
    fn test_transaction_verification() {
        let (private_key, from) = generate_account();
        let (_, to) = generate_account();
        let mut tx = Transaction::new(from, to, 1000, 1);

        tx.sign(&private_key).unwrap();
        assert!(tx.verify());
    }

    #[test]
    fn test_transaction_validation() {
        let (private_key, from) = generate_account();
        let (_, to) = generate_account();
        let mut tx = Transaction::new(from, to, 1000, 1);

        tx.sign(&private_key).unwrap();
        assert!(tx.is_valid());
    }

    #[test]
    fn test_invalid_transaction_zero_amount() {
        let (private_key, from) = generate_account();
        let (_, to) = generate_account();
        let mut tx = Transaction::new(from, to, 0, 1);

        tx.sign(&private_key).unwrap();
        assert!(!tx.is_valid());
    }

    #[test]
    fn test_invalid_transaction_same_sender_recipient() {
        let (private_key, addr) = generate_account();
        let mut tx = Transaction::new(addr, addr, 1000, 1);

        tx.sign(&private_key).unwrap();
        assert!(!tx.is_valid());
    }
}
