use serde::{Deserialize, Serialize};
use crate::{Error, Result, crypto::Hash, PublicKeyBytes, SignatureBytes};

/// IPPAN Address type with base58i encoding
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address(String);

impl Address {
    /// Create a new address from a base58i string
    pub fn new(addr: String) -> Result<Self> {
        if !addr.starts_with('i') {
            return Err(Error::Validation("Address must start with 'i'".to_string()));
        }
        if addr.len() < 2 {
            return Err(Error::Validation("Address too short".to_string()));
        }
        Ok(Address(addr))
    }

    /// Get the address as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the address as a string (owned)
    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Transaction ID (32-byte hash)
pub type TxId = Hash;

/// Block ID (32-byte hash)
pub type BlockId = Hash;

/// Round ID (u64)
pub type RoundId = u64;

/// Transaction version
pub const TRANSACTION_VERSION: u8 = 1;

/// Maximum transaction size in bytes
pub const MAX_TRANSACTION_SIZE: usize = 185;

/// Payment Transaction structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub ver: u8,
    #[serde(with = "serde_bytes")]
    pub from_pub: PublicKeyBytes,
    #[serde(with = "serde_bytes")]
    pub to_addr: PublicKeyBytes,
    pub amount: u64,
    pub nonce: u64,
    pub ippan_time_us: u64,
    #[serde(with = "serde_bytes")]
    pub hashtimer: Hash,
    #[serde(with = "serde_bytes")]
    pub sig: SignatureBytes,
}

impl Transaction {
    /// Create a new payment transaction
    pub fn new(
        from_pub: PublicKeyBytes,
        to_addr: PublicKeyBytes,
        amount: u64,
        nonce: u64,
        ippan_time_us: u64,
        hashtimer: Hash,
        sig: SignatureBytes,
    ) -> Self {
        Self {
            ver: TRANSACTION_VERSION,
            from_pub,
            to_addr,
            amount,
            nonce,
            ippan_time_us,
            hashtimer,
            sig,
        }
    }

    /// Compute the transaction ID
    pub fn compute_id(&self) -> Result<TxId> {
        let message = self.message_to_sign()?;
        Ok(crate::crypto::blake3_hash(&message))
    }

    /// Get the message to sign
    pub fn message_to_sign(&self) -> Result<Vec<u8>> {
        let mut message = Vec::new();
        message.extend_from_slice(&self.ver.to_le_bytes());
        message.extend_from_slice(&self.from_pub);
        message.extend_from_slice(&self.to_addr);
        message.extend_from_slice(&self.amount.to_le_bytes());
        message.extend_from_slice(&self.nonce.to_le_bytes());
        message.extend_from_slice(&self.ippan_time_us.to_le_bytes());
        message.extend_from_slice(&self.hashtimer);
        Ok(message)
    }

    /// Verify the transaction signature
    pub fn verify(&self) -> Result<bool> {
        let message = self.message_to_sign()?;
        crate::crypto::KeyPair::verify(&self.from_pub, &message, &self.sig)
    }

    /// Get the transaction size in bytes
    pub fn size(&self) -> usize {
        1 + 32 + 32 + 8 + 8 + 8 + 32 + 64 // ver + from_pub + to_addr + amount + nonce + ippan_time_us + hashtimer + sig
    }

    /// Get the sort key for deterministic ordering
    pub fn get_sort_key(&self) -> Result<(Hash, Hash)> {
        let tx_id = self.compute_id()?;
        Ok((self.hashtimer, tx_id))
    }

    /// Serialize to binary
    pub fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| Error::Serialization(e.to_string()))
    }

    /// Deserialize from binary
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        bincode::deserialize(data).map_err(|e| Error::Serialization(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;

    #[test]
    fn test_address_creation() {
        let addr = Address::new("i1abc123".to_string()).unwrap();
        assert_eq!(addr.as_str(), "i1abc123");
    }

    #[test]
    fn test_address_invalid() {
        assert!(Address::new("abc123".to_string()).is_err());
        assert!(Address::new("i".to_string()).is_err());
    }

    #[test]
    fn test_transaction_creation_and_verification() {
        let keypair = KeyPair::generate();
        let to_addr = [1u8; 32];
        let amount = 1000u64;
        let nonce = 1u64;
        let ippan_time_us = 1234567890u64;
        let hashtimer = [2u8; 32];
        
        let message = {
            let mut msg = Vec::new();
            msg.extend_from_slice(&TRANSACTION_VERSION.to_le_bytes());
            msg.extend_from_slice(&keypair.public_key);
            msg.extend_from_slice(&to_addr);
            msg.extend_from_slice(&amount.to_le_bytes());
            msg.extend_from_slice(&nonce.to_le_bytes());
            msg.extend_from_slice(&ippan_time_us.to_le_bytes());
            msg.extend_from_slice(&hashtimer);
            msg
        };
        
        let signature = keypair.sign(&message).unwrap();
        
        let tx = Transaction::new(
            keypair.public_key,
            to_addr,
            amount,
            nonce,
            ippan_time_us,
            hashtimer,
            signature,
        );
        
        assert!(tx.verify().unwrap());
        assert_eq!(tx.size(), MAX_TRANSACTION_SIZE);
    }

    #[test]
    fn test_transaction_serialization() {
        let keypair = KeyPair::generate();
        let to_addr = [1u8; 32];
        let tx = Transaction::new(
            keypair.public_key,
            to_addr,
            1000,
            1,
            1234567890,
            [2u8; 32],
            [3u8; 64],
        );
        
        let serialized = tx.serialize().unwrap();
        let deserialized = Transaction::deserialize(&serialized).unwrap();
        
        assert_eq!(tx.ver, deserialized.ver);
        assert_eq!(tx.from_pub, deserialized.from_pub);
        assert_eq!(tx.to_addr, deserialized.to_addr);
        assert_eq!(tx.amount, deserialized.amount);
        assert_eq!(tx.nonce, deserialized.nonce);
        assert_eq!(tx.ippan_time_us, deserialized.ippan_time_us);
        assert_eq!(tx.hashtimer, deserialized.hashtimer);
        assert_eq!(tx.sig, deserialized.sig);
    }
}
