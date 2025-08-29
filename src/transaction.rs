use crate::crypto::{self, Hash, KeyPair, PublicKeyBytes, SignatureBytes};
use crate::time::IppanTime;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const TRANSACTION_VERSION: u8 = 1;
pub const MAX_TRANSACTION_SIZE: usize = 185;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub version: u8,
    pub from_pub: PublicKeyBytes,
    pub to_addr: PublicKeyBytes, // Using public key as address for simplicity
    pub amount: u64,
    pub nonce: u64,
    pub ippan_time_us: u64,
    pub hash_timer: Hash,
    pub signature: SignatureBytes,
}

impl Transaction {
    pub fn new(
        from_keypair: &KeyPair,
        to_addr: PublicKeyBytes,
        amount: u64,
        nonce: u64,
        ippan_time: Arc<IppanTime>,
    ) -> Result<Self, crate::Error> {
        let version = TRANSACTION_VERSION;
        let from_pub = from_keypair.public_key;
        let ippan_time_us = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(ippan_time.ippan_time_us())
        });

        // Create transaction without signature first
        let mut tx = Self {
            version,
            from_pub,
            to_addr,
            amount,
            nonce,
            ippan_time_us,
            hash_timer: [0u8; 32], // Will be computed after signature
            signature: [0u8; 64],  // Will be computed
        };

        // Generate entropy for hash timer
        let entropy = rand::random::<[u8; 16]>();
        
        // Compute transaction ID (hash of all fields except signature)
        let tx_id = tx.compute_id()?;
        
        // Generate hash timer
        tx.hash_timer = crypto::generate_hash_timer(ippan_time_us, &entropy, &tx_id);
        
        // Sign the transaction
        let message = tx.message_to_sign()?;
        tx.signature = from_keypair.sign(&message)?;

        Ok(tx)
    }

    pub fn compute_id(&self) -> Result<Hash, crate::Error> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.version.to_le_bytes());
        data.extend_from_slice(&self.from_pub);
        data.extend_from_slice(&self.to_addr);
        data.extend_from_slice(&self.amount.to_le_bytes());
        data.extend_from_slice(&self.nonce.to_le_bytes());
        data.extend_from_slice(&self.ippan_time_us.to_le_bytes());
        data.extend_from_slice(&self.hash_timer);
        
        Ok(crypto::hash(&data))
    }

    pub fn message_to_sign(&self) -> Result<Vec<u8>, crate::Error> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.version.to_le_bytes());
        data.extend_from_slice(&self.from_pub);
        data.extend_from_slice(&self.to_addr);
        data.extend_from_slice(&self.amount.to_le_bytes());
        data.extend_from_slice(&self.nonce.to_le_bytes());
        data.extend_from_slice(&self.ippan_time_us.to_le_bytes());
        data.extend_from_slice(&self.hash_timer);
        
        Ok(data)
    }

    pub fn verify(&self) -> Result<bool, crate::Error> {
        // Check version
        if self.version != TRANSACTION_VERSION {
            return Err(crate::Error::Transaction("Invalid version".to_string()));
        }

        // Check size
        let serialized = bincode::serialize(self)
            .map_err(|e| crate::Error::Serialization(e.to_string()))?;
        if serialized.len() > MAX_TRANSACTION_SIZE {
            return Err(crate::Error::Transaction(format!(
                "Transaction too large: {} bytes (max: {})",
                serialized.len(),
                MAX_TRANSACTION_SIZE
            )));
        }

        // Verify signature
        let message = self.message_to_sign()?;
        let is_valid = KeyPair::verify(&self.from_pub, &message, &self.signature)?;
        
        if !is_valid {
            return Err(crate::Error::Transaction("Invalid signature".to_string()));
        }

        Ok(true)
    }

    pub fn serialize(&self) -> Result<Vec<u8>, crate::Error> {
        bincode::serialize(self).map_err(|e| crate::Error::Serialization(e.to_string()))
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, crate::Error> {
        bincode::deserialize(data).map_err(|e| crate::Error::Serialization(e.to_string()))
    }

    pub fn size(&self) -> Result<usize, crate::Error> {
        Ok(self.serialize()?.len())
    }

    pub fn get_sort_key(&self) -> Result<(Hash, Hash), crate::Error> {
        let tx_id = self.compute_id()?;
        Ok((self.hash_timer, tx_id))
    }
}

impl PartialEq for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.compute_id().unwrap_or([0u8; 32]) == other.compute_id().unwrap_or([0u8; 32])
    }
}

impl Eq for Transaction {}

impl std::hash::Hash for Transaction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if let Ok(tx_id) = self.compute_id() {
            tx_id.hash(state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;

    #[tokio::test]
    async fn test_transaction_creation() {
        let keypair = KeyPair::generate();
        let recipient = KeyPair::generate();
        let ippan_time = Arc::new(IppanTime::new());
        
        let tx = Transaction::new(
            &keypair,
            recipient.public_key,
            1000,
            1,
            ippan_time,
        ).unwrap();
        
        assert_eq!(tx.version, TRANSACTION_VERSION);
        assert_eq!(tx.from_pub, keypair.public_key);
        assert_eq!(tx.to_addr, recipient.public_key);
        assert_eq!(tx.amount, 1000);
        assert_eq!(tx.nonce, 1);
    }

    #[tokio::test]
    async fn test_transaction_verification() {
        let keypair = KeyPair::generate();
        let recipient = KeyPair::generate();
        let ippan_time = Arc::new(IppanTime::new());
        
        let tx = Transaction::new(
            &keypair,
            recipient.public_key,
            1000,
            1,
            ippan_time,
        ).unwrap();
        
        assert!(tx.verify().unwrap());
    }

    #[tokio::test]
    async fn test_transaction_serialization() {
        let keypair = KeyPair::generate();
        let recipient = KeyPair::generate();
        let ippan_time = Arc::new(IppanTime::new());
        
        let tx = Transaction::new(
            &keypair,
            recipient.public_key,
            1000,
            1,
            ippan_time,
        ).unwrap();
        
        let serialized = tx.serialize().unwrap();
        let deserialized = Transaction::deserialize(&serialized).unwrap();
        
        assert_eq!(tx.from_pub, deserialized.from_pub);
        assert_eq!(tx.to_addr, deserialized.to_addr);
        assert_eq!(tx.amount, deserialized.amount);
        assert_eq!(tx.nonce, deserialized.nonce);
    }

    #[tokio::test]
    async fn test_transaction_size_limit() {
        let keypair = KeyPair::generate();
        let recipient = KeyPair::generate();
        let ippan_time = Arc::new(IppanTime::new());
        
        let tx = Transaction::new(
            &keypair,
            recipient.public_key,
            1000,
            1,
            ippan_time,
        ).unwrap();
        
        let size = tx.size().unwrap();
        assert!(size <= MAX_TRANSACTION_SIZE);
    }

    #[tokio::test]
    async fn test_sort_key_generation() {
        let keypair = KeyPair::generate();
        let recipient = KeyPair::generate();
        let ippan_time = Arc::new(IppanTime::new());
        
        let tx = Transaction::new(
            &keypair,
            recipient.public_key,
            1000,
            1,
            ippan_time,
        ).unwrap();
        
        let (hash_timer, tx_id) = tx.get_sort_key().unwrap();
        assert_eq!(hash_timer.len(), 32);
        assert_eq!(tx_id.len(), 32);
    }
}
