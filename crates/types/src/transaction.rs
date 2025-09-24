use crate::hashtimer::{random_nonce, HashTimer, IppanTimeMicros};
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
        let nonce_bytes = random_nonce();
        let node_id = b"local_node"; // In real implementation, this would be the actual node ID

        // Create HashTimer for this transaction
        let payload = Self::create_payload(&from, &to, amount, nonce);
        let hashtimer = HashTimer::now_tx("transaction", &payload, &nonce_bytes, node_id);

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

    /// Recompute the transaction identifier from its contents.
    pub fn refresh_id(&mut self) {
        self.id = self.compute_hash();
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

    /// Sign the transaction (placeholder implementation)
    pub fn sign(&mut self, private_key: &[u8; 32]) -> Result<(), String> {
        // In a real implementation, this would use proper cryptographic signing
        // For now, we'll create a deterministic signature based on the transaction data
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.from);
        hasher.update(&self.to);
        hasher.update(&self.amount.to_be_bytes());
        hasher.update(&self.nonce.to_be_bytes());
        hasher.update(self.hashtimer.to_hex().as_bytes());
        // The placeholder implementation doesn't perform real cryptographic signing.
        // Keep the parameter to maintain API compatibility but avoid using it so that
        // verification can deterministically recompute the same signature from the
        // transaction contents alone.
        let _ = private_key;

        let mut output = [0u8; 64];
        hasher.finalize_xof().fill(&mut output);
        self.signature.copy_from_slice(&output);
        self.refresh_id();

        Ok(())
    }

    /// Verify the transaction signature
    pub fn verify(&self) -> bool {
        // In a real implementation, this would verify the cryptographic signature
        // For now, we'll verify that the signature is consistent with the transaction data
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.from);
        hasher.update(&self.to);
        hasher.update(&self.amount.to_be_bytes());
        hasher.update(&self.nonce.to_be_bytes());
        hasher.update(self.hashtimer.to_hex().as_bytes());

        let mut expected_signature = [0u8; 64];
        hasher.finalize_xof().fill(&mut expected_signature);

        self.signature == expected_signature
    }

    /// Get transaction hash
    pub fn hash(&self) -> [u8; 32] {
        self.compute_hash()
    }

    fn compute_hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.from);
        hasher.update(&self.to);
        hasher.update(&self.amount.to_be_bytes());
        hasher.update(&self.nonce.to_be_bytes());
        hasher.update(&self.signature);
        hasher.update(self.hashtimer.to_hex().as_bytes());

        let hash = hasher.finalize();
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash.as_bytes()[0..32]);
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

        // Ensure the ID matches the computed hash
        if self.id != self.compute_hash() {
            return false;
        }

        // Verify HashTimer is valid
        self.hashtimer.time().0 <= IppanTimeMicros::now().0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_creation() {
        let from = [1u8; 32];
        let to = [2u8; 32];
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
        let mut tx = Transaction::new([1u8; 32], [2u8; 32], 1000, 1);
        let private_key = [3u8; 32];

        let result = tx.sign(&private_key);
        assert!(result.is_ok());
        assert_ne!(tx.signature, [0u8; 64]);
        assert_ne!(tx.id, [0u8; 32]);
    }

    #[test]
    fn test_transaction_verification() {
        let mut tx = Transaction::new([1u8; 32], [2u8; 32], 1000, 1);
        let private_key = [3u8; 32];

        tx.sign(&private_key).unwrap();
        assert!(tx.verify());
    }

    #[test]
    fn test_transaction_validation() {
        let mut tx = Transaction::new([1u8; 32], [2u8; 32], 1000, 1);
        let private_key = [3u8; 32];

        tx.sign(&private_key).unwrap();
        assert!(tx.is_valid());
    }

    #[test]
    fn test_invalid_transaction_zero_amount() {
        let mut tx = Transaction::new([1u8; 32], [2u8; 32], 0, 1);
        let private_key = [3u8; 32];

        tx.sign(&private_key).unwrap();
        assert!(!tx.is_valid());
    }

    #[test]
    fn test_invalid_transaction_same_sender_recipient() {
        let addr = [1u8; 32];
        let mut tx = Transaction::new(addr, addr, 1000, 1);
        let private_key = [3u8; 32];

        tx.sign(&private_key).unwrap();
        assert!(!tx.is_valid());
    }
}
