use crate::Result;
use ed25519_dalek::{SecretKey, PublicKey, Signer, Verifier, Signature};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::fs;
use std::path::Path;

/// Ed25519 wallet for key management
#[derive(Clone)]
pub struct Ed25519Wallet {
    /// Secret key
    secret_key: SecretKey,
    /// Public key
    public_key: PublicKey,
    /// Wallet file path
    wallet_path: String,
    /// Whether wallet is encrypted
    encrypted: bool,
}

/// Wallet data structure for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WalletData {
    /// Secret key bytes
    secret_key: Vec<u8>,
    /// Public key bytes
    public_key: Vec<u8>,
    /// Creation timestamp
    created_at: u64,
    /// Version
    version: u32,
}

impl Ed25519Wallet {
    /// Create a new Ed25519 wallet
    pub async fn new(wallet_path: &str, encrypted: bool) -> Result<Self> {
        let path = Path::new(wallet_path);
        
        if path.exists() {
            // Load existing wallet
            Self::load(wallet_path, encrypted).await
        } else {
            // Create new wallet
            Self::create_new(wallet_path, encrypted).await
        }
    }

    /// Create a new wallet with fresh keys
    async fn create_new(wallet_path: &str, encrypted: bool) -> Result<Self> {
        // Generate new keypair
        let secret_key = SecretKey::generate(&mut rand::thread_rng());
        let public_key = PublicKey::from(&secret_key);

        let wallet = Self {
            secret_key,
            public_key,
            wallet_path: wallet_path.to_string(),
            encrypted,
        };

        // Save wallet to file
        wallet.save().await?;

        Ok(wallet)
    }

    /// Load existing wallet from file
    async fn load(wallet_path: &str, encrypted: bool) -> Result<Self> {
        let data = fs::read(wallet_path)
            .map_err(|e| crate::IppanError::Wallet(format!("Failed to read wallet: {}", e)))?;

        let wallet_data: WalletData = if encrypted {
            // TODO: Implement wallet encryption/decryption
            bincode::deserialize(&data)
                .map_err(|e| crate::IppanError::Wallet(format!("Failed to deserialize wallet: {}", e)))?
        } else {
            bincode::deserialize(&data)
                .map_err(|e| crate::IppanError::Wallet(format!("Failed to deserialize wallet: {}", e)))?
        };

        let secret_key = SecretKey::from_bytes(&wallet_data.secret_key)
            .map_err(|e| crate::IppanError::Wallet(format!("Invalid secret key: {}", e)))?;
        let public_key = PublicKey::from_bytes(&wallet_data.public_key)
            .map_err(|e| crate::IppanError::Wallet(format!("Invalid public key: {}", e)))?;

        Ok(Self {
            secret_key,
            public_key,
            wallet_path: wallet_path.to_string(),
            encrypted,
        })
    }

    /// Save wallet to file
    async fn save(&self) -> Result<()> {
        let wallet_data = WalletData {
            secret_key: self.secret_key.to_bytes().to_vec(),
            public_key: self.public_key.to_bytes().to_vec(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            version: 1,
        };

        let data = if self.encrypted {
            // TODO: Implement wallet encryption
            bincode::serialize(&wallet_data)
                .map_err(|e| crate::IppanError::Wallet(format!("Failed to serialize wallet: {}", e)))?
        } else {
            bincode::serialize(&wallet_data)
                .map_err(|e| crate::IppanError::Wallet(format!("Failed to serialize wallet: {}", e)))?
        };

        fs::write(&self.wallet_path, data)
            .map_err(|e| crate::IppanError::Wallet(format!("Failed to write wallet: {}", e)))?;

        Ok(())
    }

    /// Get public key
    pub fn get_public_key(&self) -> [u8; 32] {
        self.public_key.to_bytes()
    }

    /// Get secret key (use with caution)
    pub fn get_secret_key(&self) -> [u8; 32] {
        self.secret_key.to_bytes()
    }

    /// Get node ID (derived from public key)
    pub fn get_node_id(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.public_key.to_bytes());
        hasher.finalize().into()
    }

    /// Sign data
    pub async fn sign(&self, data: &[u8]) -> Result<[u8; 64]> {
        let signature = self.secret_key.sign(data);
        Ok(signature.to_bytes())
    }

    /// Verify signature
    pub fn verify(&self, data: &[u8], signature: &[u8; 64]) -> Result<bool> {
        let signature = match Signature::from_bytes(signature) {
            Ok(sig) => sig,
            Err(_) => return Ok(false),
        };
        
        match self.public_key.verify(data, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Verify signature with a specific public key
    pub fn verify_with_key(&self, data: &[u8], signature: &[u8; 64], public_key: &[u8; 32]) -> Result<bool> {
        let public_key = match PublicKey::from_bytes(public_key) {
            Ok(key) => key,
            Err(_) => return Ok(false),
        };
        
        let signature = match Signature::from_bytes(signature) {
            Ok(sig) => sig,
            Err(_) => return Ok(false),
        };
        
        match public_key.verify(data, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Export wallet data
    pub async fn export(&self) -> Result<Vec<u8>> {
        let wallet_data = WalletData {
            secret_key: self.secret_key.to_bytes().to_vec(),
            public_key: self.public_key.to_bytes().to_vec(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            version: 1,
        };

        bincode::serialize(&wallet_data)
            .map_err(|e| crate::IppanError::Wallet(format!("Failed to export wallet: {}", e)))
    }

    /// Import wallet data
    pub async fn import(&mut self, data: &[u8]) -> Result<()> {
        let wallet_data: WalletData = bincode::deserialize(data)
            .map_err(|e| crate::IppanError::Wallet(format!("Failed to import wallet: {}", e)))?;

        let secret_key = SecretKey::from_bytes(&wallet_data.secret_key)
            .map_err(|e| crate::IppanError::Wallet(format!("Invalid secret key: {}", e)))?;
        let public_key = PublicKey::from_bytes(&wallet_data.public_key)
            .map_err(|e| crate::IppanError::Wallet(format!("Invalid public key: {}", e)))?;

        self.secret_key = secret_key;
        self.public_key = public_key;

        // Save the imported wallet
        self.save().await?;

        Ok(())
    }

    /// Generate a deterministic key from a seed
    pub async fn from_seed(seed: &[u8]) -> Result<Self> {
        let mut hasher = Sha256::new();
        hasher.update(seed);
        let seed_hash = hasher.finalize();

        let secret_key = SecretKey::from_bytes(&seed_hash)
            .map_err(|e| crate::IppanError::Wallet(format!("Invalid seed: {}", e)))?;
        let public_key = PublicKey::from(&secret_key);

        Ok(Self {
            secret_key,
            public_key,
            wallet_path: String::new(), // Not saved to file
            encrypted: false,
        })
    }

    /// Get wallet information
    pub fn get_info(&self) -> WalletInfo {
        WalletInfo {
            public_key: self.public_key.to_bytes(),
            node_id: self.get_node_id(),
            encrypted: self.encrypted,
            path: self.wallet_path.clone(),
        }
    }
}

/// Wallet information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    /// Public key
    pub public_key: [u8; 32],
    /// Node ID
    pub node_id: [u8; 32],
    /// Whether wallet is encrypted
    pub encrypted: bool,
    /// Wallet file path
    pub path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wallet_creation() {
        let wallet = Ed25519Wallet::new("./test_wallet.dat", false).await.unwrap();
        
        let public_key = wallet.get_public_key();
        assert_ne!(public_key, [0u8; 32]);
        
        let node_id = wallet.get_node_id();
        assert_ne!(node_id, [0u8; 32]);
        
        // Clean up
        let _ = fs::remove_file("./test_wallet.dat");
    }

    #[tokio::test]
    async fn test_signing_and_verification() {
        let wallet = Ed25519Wallet::new("./test_wallet2.dat", false).await.unwrap();
        
        let data = b"Test data for signing";
        let signature = wallet.sign(data).await.unwrap();
        
        let is_valid = wallet.verify(data, &signature).unwrap();
        assert!(is_valid);
        
        // Test with wrong data
        let wrong_data = b"Wrong data";
        let is_valid = wallet.verify(wrong_data, &signature).unwrap();
        assert!(!is_valid);
        
        // Clean up
        let _ = fs::remove_file("./test_wallet2.dat");
    }

    #[tokio::test]
    async fn test_deterministic_wallet() {
        let seed = b"test seed for deterministic wallet";
        let wallet1 = Ed25519Wallet::from_seed(seed).await.unwrap();
        let wallet2 = Ed25519Wallet::from_seed(seed).await.unwrap();
        
        assert_eq!(wallet1.get_public_key(), wallet2.get_public_key());
        assert_eq!(wallet1.get_node_id(), wallet2.get_node_id());
    }

    #[tokio::test]
    async fn test_export_import() {
        let wallet1 = Ed25519Wallet::new("./test_wallet3.dat", false).await.unwrap();
        let export_data = wallet1.export().await.unwrap();
        
        let mut wallet2 = Ed25519Wallet::new("./test_wallet4.dat", false).await.unwrap();
        wallet2.import(&export_data).await.unwrap();
        
        assert_eq!(wallet1.get_public_key(), wallet2.get_public_key());
        assert_eq!(wallet1.get_node_id(), wallet2.get_node_id());
        
        // Clean up
        let _ = fs::remove_file("./test_wallet3.dat");
        let _ = fs::remove_file("./test_wallet4.dat");
    }
}
