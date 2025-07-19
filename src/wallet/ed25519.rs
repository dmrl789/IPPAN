//! Ed25519 key management for IPPAN wallet

use crate::Result;
use crate::utils::address::{generate_ippan_address, validate_ippan_address};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Ed25519 key pair with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ed25519KeyPair {
    /// Public key
    pub public_key: [u8; 32],
    /// Private key (encrypted in production)
    pub private_key: [u8; 32],
    /// IPPAN address derived from public key
    pub address: String,
    /// Key creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Key label/name
    pub label: Option<String>,
    /// Whether this key is currently active
    pub is_active: bool,
}

/// Ed25519 key manager
pub struct Ed25519Manager {
    /// Key pairs indexed by address
    keys: HashMap<String, Ed25519KeyPair>,
    /// Default key address
    default_key: Option<String>,
    /// Key storage path
    storage_path: String,
    /// Whether keys are encrypted
    encrypted: bool,
}

impl Ed25519Manager {
    /// Create a new Ed25519 manager
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            default_key: None,
            storage_path: "keys/".to_string(),
            encrypted: false, // TODO: Enable encryption in production
        }
    }

    /// Initialize the key manager
    pub async fn initialize(&mut self) -> Result<()> {
        // Create storage directory if it doesn't exist
        tokio::fs::create_dir_all(&self.storage_path).await?;
        
        // Load existing keys
        self.load_keys().await?;
        
        // Generate default key if none exists
        if self.keys.is_empty() {
            self.generate_key_pair("default".to_string()).await?;
        }
        
        Ok(())
    }

    /// Shutdown the key manager
    pub async fn shutdown(&mut self) -> Result<()> {
        // Save keys to storage
        self.save_keys().await?;
        Ok(())
    }

    /// Generate a new Ed25519 key pair
    pub async fn generate_key_pair(&mut self, label: String) -> Result<Ed25519KeyPair> {
        let mut rng = rand::thread_rng();
        let mut signing_key_bytes = [0u8; 32];
        rng.fill_bytes(&mut signing_key_bytes);
        
        let signing_key = SigningKey::from_bytes(&signing_key_bytes);
        let verifying_key = signing_key.verifying_key();
        
        let public_key = verifying_key.to_bytes();
        let address = generate_ippan_address(&public_key);
        
        let key_pair = Ed25519KeyPair {
            public_key,
            private_key: signing_key_bytes,
            address: address.clone(),
            created_at: chrono::Utc::now(),
            label: Some(label),
            is_active: true,
        };
        
        self.keys.insert(address.clone(), key_pair.clone());
        
        // Set as default if it's the first key
        if self.default_key.is_none() {
            self.default_key = Some(address);
        }
        
        Ok(key_pair)
    }

    /// Get a key pair by address
    pub fn get_key_pair(&self, address: &str) -> Option<&Ed25519KeyPair> {
        self.keys.get(address)
    }

    /// Get the default key pair
    pub fn get_default_key_pair(&self) -> Option<&Ed25519KeyPair> {
        self.default_key.as_ref().and_then(|addr| self.keys.get(addr))
    }

    /// Set the default key
    pub fn set_default_key(&mut self, address: &str) -> Result<()> {
        if self.keys.contains_key(address) {
            self.default_key = Some(address.to_string());
            Ok(())
        } else {
            Err(crate::error::IppanError::Validation(
                format!("Key not found: {}", address)
            ))
        }
    }

    /// Sign data with a key
    pub fn sign_data(&self, address: &str, data: &[u8]) -> Result<Signature> {
        let key_pair = self.get_key_pair(address)
            .ok_or_else(|| crate::error::IppanError::Validation(
                format!("Key not found: {}", address)
            ))?;
        
        let signing_key = SigningKey::from_bytes(&key_pair.private_key);
        Ok(signing_key.sign(data))
    }

    /// Verify a signature
    pub fn verify_signature(&self, address: &str, data: &[u8], signature: &Signature) -> Result<bool> {
        let key_pair = self.get_key_pair(address)
            .ok_or_else(|| crate::error::IppanError::Validation(
                format!("Key not found: {}", address)
            ))?;
        
        let verifying_key = VerifyingKey::from_bytes(&key_pair.public_key)
            .map_err(|e| crate::error::IppanError::Validation(
                format!("Invalid public key: {}", e)
            ))?;
        
        Ok(verifying_key.verify(data, signature).is_ok())
    }

    /// Get all key pairs
    pub fn get_all_keys(&self) -> Vec<&Ed25519KeyPair> {
        self.keys.values().collect()
    }

    /// Get active key pairs
    pub fn get_active_keys(&self) -> Vec<&Ed25519KeyPair> {
        self.keys.values().filter(|k| k.is_active).collect()
    }

    /// Deactivate a key
    pub fn deactivate_key(&mut self, address: &str) -> Result<()> {
        if let Some(key_pair) = self.keys.get_mut(address) {
            key_pair.is_active = false;
            Ok(())
        } else {
            Err(crate::error::IppanError::Validation(
                format!("Key not found: {}", address)
            ))
        }
    }

    /// Activate a key
    pub fn activate_key(&mut self, address: &str) -> Result<()> {
        if let Some(key_pair) = self.keys.get_mut(address) {
            key_pair.is_active = true;
            Ok(())
        } else {
            Err(crate::error::IppanError::Validation(
                format!("Key not found: {}", address)
            ))
        }
    }

    /// Validate an IPPAN address
    pub fn validate_address(&self, address: &str) -> bool {
        validate_ippan_address(address).is_ok()
    }

    /// Get key statistics
    pub fn get_key_stats(&self) -> KeyStats {
        let total_keys = self.keys.len();
        let active_keys = self.keys.values().filter(|k| k.is_active).count();
        let default_key = self.default_key.clone();
        
        KeyStats {
            total_keys,
            active_keys,
            default_key,
        }
    }

    /// Load keys from storage
    async fn load_keys(&mut self) -> Result<()> {
        let keys_file = format!("{}keys.json", self.storage_path);
        
        if let Ok(contents) = tokio::fs::read_to_string(&keys_file).await {
            let keys: HashMap<String, Ed25519KeyPair> = serde_json::from_str(&contents)?;
            self.keys = keys;
        }
        
        Ok(())
    }

    /// Save keys to storage
    async fn save_keys(&self) -> Result<()> {
        let keys_file = format!("{}keys.json", self.storage_path);
        let contents = serde_json::to_string_pretty(&self.keys)?;
        tokio::fs::write(&keys_file, contents).await?;
        Ok(())
    }
}

/// Key statistics
#[derive(Debug, Serialize)]
pub struct KeyStats {
    pub total_keys: usize,
    pub active_keys: usize,
    pub default_key: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_key_manager_creation() {
        let mut manager = Ed25519Manager::new();
        manager.initialize().await.unwrap();
        
        assert!(manager.keys.is_empty() || manager.default_key.is_some());
    }

    #[tokio::test]
    async fn test_key_generation() {
        let mut manager = Ed25519Manager::new();
        manager.initialize().await.unwrap();
        
        let key_pair = manager.generate_key_pair("test_key".to_string()).await.unwrap();
        
        assert!(manager.validate_address(&key_pair.address));
        assert_eq!(key_pair.label, Some("test_key".to_string()));
        assert!(key_pair.is_active);
    }

    #[tokio::test]
    async fn test_signing_and_verification() {
        let mut manager = Ed25519Manager::new();
        manager.initialize().await.unwrap();
        
        let key_pair = manager.generate_key_pair("test_key".to_string()).await.unwrap();
        let data = b"test data";
        
        let signature = manager.sign_data(&key_pair.address, data).unwrap();
        let is_valid = manager.verify_signature(&key_pair.address, data, &signature).unwrap();
        
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_key_activation_deactivation() {
        let mut manager = Ed25519Manager::new();
        manager.initialize().await.unwrap();
        
        let key_pair = manager.generate_key_pair("test_key".to_string()).await.unwrap();
        
        // Deactivate
        manager.deactivate_key(&key_pair.address).unwrap();
        assert!(!manager.get_key_pair(&key_pair.address).unwrap().is_active);
        
        // Activate
        manager.activate_key(&key_pair.address).unwrap();
        assert!(manager.get_key_pair(&key_pair.address).unwrap().is_active);
    }
}
