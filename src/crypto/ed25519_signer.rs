//! Ed25519 transaction signing utility for IPPAN
//! 
//! Provides canonical transaction signing and verification

use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier, SecretKey};
use rand::{rngs::OsRng, RngCore, Rng};

/// Ed25519 keypair for transaction signing
#[derive(Debug, Clone)]
pub struct Ed25519Keypair {
    pub private_key: SigningKey,
    pub public_key: VerifyingKey,
}

impl Ed25519Keypair {
    /// Generate a new random keypair
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let mut secret_bytes = [0u8; 32];
        csprng.fill(&mut secret_bytes);
        let private_key = SigningKey::from_bytes(&secret_bytes);
        let public_key = private_key.verifying_key();
        
        Self {
            private_key,
            public_key,
        }
    }
    
    /// Create keypair from private key bytes
    pub fn from_private_key_bytes(bytes: &[u8; 32]) -> Result<Self, String> {
        let private_key = SigningKey::from_bytes(bytes);
        let public_key = private_key.verifying_key();
        
        Ok(Self {
            private_key,
            public_key,
        })
    }
    
    /// Get public key as hex string
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public_key.as_bytes())
    }
    
    /// Get private key as hex string
    pub fn private_key_hex(&self) -> String {
        hex::encode(self.private_key.as_bytes())
    }
}

/// Canonical transaction for signing
#[derive(Debug, Serialize, Deserialize)]
pub struct CanonicalTransaction {
    pub chain_id: String,
    pub from: String,
    pub to: String,
    pub amount: String,
    pub fee: String,
    pub nonce: u64,
    pub timestamp: String,
}

impl CanonicalTransaction {
    /// Create canonical transaction
    pub fn new(
        chain_id: String,
        from: String,
        to: String,
        amount: String,
        fee: String,
        nonce: u64,
        timestamp: String,
    ) -> Self {
        Self {
            chain_id,
            from,
            to,
            amount,
            fee,
            nonce,
            timestamp,
        }
    }
    
    /// Serialize to canonical JSON (stable field order)
    pub fn to_canonical_json(&self) -> Result<String, String> {
        serde_json::to_string(self)
            .map_err(|e| format!("Failed to serialize transaction: {}", e))
    }
    
    /// Hash the canonical JSON
    pub fn hash(&self) -> Result<[u8; 32], String> {
        let json = self.to_canonical_json()?;
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        let hash = hasher.finalize();
        Ok(hash.into())
    }
}

/// Signed transaction
#[derive(Debug, Serialize, Deserialize)]
pub struct SignedTransaction {
    pub tx: CanonicalTransaction,
    pub signature: String,
    pub pubkey: String,
}

impl SignedTransaction {
    /// Create signed transaction
    pub fn new(tx: CanonicalTransaction, signature: String, pubkey: String) -> Self {
        Self {
            tx,
            signature,
            pubkey,
        }
    }
    
    /// Verify the signature
    pub fn verify(&self) -> Result<bool, String> {
        let msg_hash = self.tx.hash()?;
        let signature_bytes = hex::decode(&self.signature)
            .map_err(|e| format!("Invalid signature hex: {}", e))?;
        
        if signature_bytes.len() != 64 {
            return Err("Invalid signature length".to_string());
        }
        
        let mut sig_array = [0u8; 64];
        sig_array.copy_from_slice(&signature_bytes);
        let signature = Signature::from_bytes(&sig_array);
        
        let pubkey_bytes = hex::decode(&self.pubkey)
            .map_err(|e| format!("Invalid pubkey hex: {}", e))?;
        
        if pubkey_bytes.len() != 32 {
            return Err("Invalid pubkey length".to_string());
        }
        
        let mut pubkey_array = [0u8; 32];
        pubkey_array.copy_from_slice(&pubkey_bytes);
        let pubkey = VerifyingKey::from_bytes(&pubkey_array)
            .map_err(|e| format!("Invalid pubkey: {}", e))?;
        
        Ok(pubkey.verify(&msg_hash, &signature).is_ok())
    }
}

/// Transaction signer
pub struct TransactionSigner {
    keypair: Ed25519Keypair,
}

impl TransactionSigner {
    /// Create new signer with keypair
    pub fn new(keypair: Ed25519Keypair) -> Self {
        Self { keypair }
    }
    
    /// Generate new signer with random keypair
    pub fn generate() -> Self {
        Self {
            keypair: Ed25519Keypair::generate(),
        }
    }
    
    /// Sign a transaction
    pub fn sign_transaction(&self, tx: CanonicalTransaction) -> Result<SignedTransaction, String> {
        let msg_hash = tx.hash()?;
        let signature = self.keypair.private_key.sign(&msg_hash);
        let signature_hex = hex::encode(signature.to_bytes());
        let pubkey_hex = self.keypair.public_key_hex();
        
        Ok(SignedTransaction::new(tx, signature_hex, pubkey_hex))
    }
    
    /// Get public key
    pub fn public_key(&self) -> &VerifyingKey {
        &self.keypair.public_key
    }
    
    /// Get public key as hex
    pub fn public_key_hex(&self) -> String {
        self.keypair.public_key_hex()
    }
}

/// Utility functions for transaction signing
pub mod utils {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    /// Create a test transaction for the funded sender
    pub fn create_test_transaction(
        to: &str,
        amount: &str,
        fee: &str,
        nonce: u64,
    ) -> CanonicalTransaction {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        CanonicalTransaction::new(
            "ippan-devnet-001".to_string(),
            "iSender1111111111111111111111111111111111111".to_string(),
            to.to_string(),
            amount.to_string(),
            fee.to_string(),
            nonce,
            timestamp.to_string(),
        )
    }
    
    /// Generate a test keypair for the funded sender
    pub fn generate_test_keypair() -> Ed25519Keypair {
        // Use a deterministic seed for testing
        let seed = [1u8; 32]; // This should be replaced with actual private key
        Ed25519Keypair::from_private_key_bytes(&seed).unwrap()
    }
    
    /// Create and sign a test transaction
    pub fn create_signed_test_transaction(
        to: &str,
        amount: &str,
        fee: &str,
        nonce: u64,
    ) -> Result<SignedTransaction, String> {
        let tx = create_test_transaction(to, amount, fee, nonce);
        let signer = TransactionSigner::new(generate_test_keypair());
        signer.sign_transaction(tx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_keypair_generation() {
        let keypair = Ed25519Keypair::generate();
        assert_eq!(keypair.public_key.as_bytes().len(), 32);
        assert_eq!(keypair.private_key.as_bytes().len(), 32);
    }
    
    #[test]
    fn test_transaction_signing() {
        let keypair = Ed25519Keypair::generate();
        let signer = TransactionSigner::new(keypair);
        
        let tx = CanonicalTransaction::new(
            "test-chain".to_string(),
            "alice".to_string(),
            "bob".to_string(),
            "1000".to_string(),
            "10".to_string(),
            1,
            "1234567890".to_string(),
        );
        
        let signed_tx = signer.sign_transaction(tx).unwrap();
        assert!(signed_tx.verify().unwrap());
    }
    
    #[test]
    fn test_canonical_json() {
        let tx = CanonicalTransaction::new(
            "test-chain".to_string(),
            "alice".to_string(),
            "bob".to_string(),
            "1000".to_string(),
            "10".to_string(),
            1,
            "1234567890".to_string(),
        );
        
        let json = tx.to_canonical_json().unwrap();
        assert!(json.contains("chain_id"));
        assert!(json.contains("from"));
        assert!(json.contains("to"));
        assert!(json.contains("amount"));
        assert!(json.contains("fee"));
        assert!(json.contains("nonce"));
        assert!(json.contains("timestamp"));
    }
}

