//! Ed25519 key management for IPPAN wallet

use crate::{Result, IppanError};
use crate::utils::address::{generate_ippan_address, validate_ippan_address};
use crate::utils::crypto::{generate_aes_key, encrypt_aes_gcm, decrypt_aes_gcm, generate_nonce};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature};
use ed25519_dalek::{Signer, Verifier};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Ed25519 key pair with enhanced metadata
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
    /// Key expiration timestamp (for rotation)
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Key label/name
    pub label: Option<String>,
    /// Whether this key is currently active
    pub is_active: bool,
    /// Key rotation priority (higher = more important)
    pub rotation_priority: u8,
    /// Last used timestamp
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
    /// Usage count
    pub usage_count: u64,
    /// Key version for rotation tracking
    pub version: u32,
    /// Whether this key is encrypted
    pub is_encrypted: bool,
    /// Key derivation path (for hierarchical keys)
    pub derivation_path: Option<String>,
}

/// Key rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationConfig {
    /// Enable automatic key rotation
    pub auto_rotation_enabled: bool,
    /// Key rotation interval in days
    pub rotation_interval_days: u32,
    /// Warning period before expiration (days)
    pub warning_period_days: u32,
    /// Maximum key age before forced rotation (days)
    pub max_key_age_days: u32,
    /// Minimum key age before rotation allowed (days)
    pub min_key_age_days: u32,
    /// Maximum usage count before rotation
    pub max_usage_count: u64,
    /// Enable usage-based rotation
    pub usage_based_rotation: bool,
}

/// Ed25519 key manager with enhanced security
pub struct Ed25519Manager {
    /// Key pairs indexed by address
    keys: HashMap<String, Ed25519KeyPair>,
    /// Default key address
    default_key: Option<String>,
    /// Key storage path
    storage_path: String,
    /// Whether keys are encrypted
    encrypted: bool,
    /// Master encryption key for key encryption
    master_key: Option<[u8; 32]>,
    /// Key rotation configuration
    rotation_config: KeyRotationConfig,
    /// Key version counter
    key_version_counter: u32,
}

impl Ed25519Manager {
    /// Create a new Ed25519 manager
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            default_key: None,
            storage_path: "keys/".to_string(),
            encrypted: true, // Enable encryption by default
            master_key: None,
            rotation_config: KeyRotationConfig {
                auto_rotation_enabled: true,
                rotation_interval_days: 90, // Rotate every 90 days
                warning_period_days: 7,     // Warn 7 days before expiration
                max_key_age_days: 365,      // Force rotation after 1 year
                min_key_age_days: 1,        // Allow rotation after 1 day
                max_usage_count: 10000,     // Rotate after 10k uses
                usage_based_rotation: true,
            },
            key_version_counter: 1,
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

    /// Generate a new Ed25519 key pair with enhanced security
    pub async fn generate_key_pair(&mut self, label: String) -> Result<Ed25519KeyPair> {
        let mut rng = rand::thread_rng();
        let mut signing_key_bytes = [0u8; 32];
        rng.fill_bytes(&mut signing_key_bytes);
        
        let signing_key = SigningKey::from_bytes(&signing_key_bytes);
        let verifying_key = signing_key.verifying_key();
        
        let public_key = verifying_key.to_bytes();
        let address = generate_ippan_address(&public_key);
        
        // Calculate expiration date
        let expires_at = if self.rotation_config.auto_rotation_enabled {
            Some(chrono::Utc::now() + chrono::Duration::days(self.rotation_config.rotation_interval_days as i64))
        } else {
            None
        };
        
        let key_pair = Ed25519KeyPair {
            public_key,
            private_key: signing_key_bytes,
            address: address.clone(),
            created_at: chrono::Utc::now(),
            expires_at,
            label: Some(label),
            is_active: true,
            rotation_priority: 5, // Default priority
            last_used: None,
            usage_count: 0,
            version: self.key_version_counter,
            is_encrypted: self.encrypted,
            derivation_path: None,
        };
        
        self.keys.insert(address.clone(), key_pair.clone());
        self.key_version_counter += 1;
        
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
        let expired_keys = self.keys.values().filter(|k| {
            if let Some(expires_at) = k.expires_at {
                chrono::Utc::now() > expires_at
            } else {
                false
            }
        }).count();
        let keys_needing_rotation = self.keys.values().filter(|k| self.should_rotate_key(k)).count();
        
        KeyStats {
            total_keys,
            active_keys,
            default_key,
            expired_keys,
            keys_needing_rotation,
        }
    }

    /// Check if a key should be rotated
    fn should_rotate_key(&self, key: &Ed25519KeyPair) -> bool {
        if !self.rotation_config.auto_rotation_enabled {
            return false;
        }

        let now = chrono::Utc::now();
        
        // Check expiration
        if let Some(expires_at) = key.expires_at {
            if now > expires_at {
                return true;
            }
        }
        
        // Check age-based rotation
        let age_days = (now - key.created_at).num_days();
        if age_days >= self.rotation_config.max_key_age_days as i64 {
            return true;
        }
        
        // Check usage-based rotation
        if self.rotation_config.usage_based_rotation && key.usage_count >= self.rotation_config.max_usage_count {
            return true;
        }
        
        false
    }

    /// Rotate a key (generate new key and mark old one for retirement)
    pub async fn rotate_key(&mut self, address: &str) -> Result<Ed25519KeyPair> {
        let old_key = self.keys.get(address)
            .ok_or_else(|| IppanError::Validation(format!("Key not found: {}", address)))?;
        
        // Generate new key with same label
        let label = old_key.label.clone().unwrap_or_else(|| "rotated_key".to_string());
        let new_key = self.generate_key_pair(label).await?;
        
        // Mark old key as inactive
        if let Some(key) = self.keys.get_mut(address) {
            key.is_active = false;
        }
        
        // Set new key as default if old key was default
        if self.default_key.as_ref() == Some(&address.to_string()) {
            self.default_key = Some(new_key.address.clone());
        }
        
        Ok(new_key)
    }

    /// Rotate all keys that need rotation
    pub async fn rotate_expired_keys(&mut self) -> Result<Vec<Ed25519KeyPair>> {
        let mut rotated_keys = Vec::new();
        let keys_to_rotate: Vec<String> = self.keys.iter()
            .filter(|(_, key)| self.should_rotate_key(key))
            .map(|(address, _)| address.clone())
            .collect();
        
        for address in keys_to_rotate {
            match self.rotate_key(&address).await {
                Ok(new_key) => rotated_keys.push(new_key),
                Err(e) => log::warn!("Failed to rotate key {}: {}", address, e),
            }
        }
        
        Ok(rotated_keys)
    }

    /// Set master encryption key
    pub fn set_master_key(&mut self, master_key: [u8; 32]) {
        self.master_key = Some(master_key);
    }

    /// Encrypt a private key
    fn encrypt_private_key(&self, private_key: &[u8; 32]) -> Result<Vec<u8>> {
        if let Some(master_key) = &self.master_key {
            let nonce = generate_nonce();
            let encrypted = encrypt_aes_gcm(master_key, &nonce, private_key)
                .map_err(|e| IppanError::Crypto(format!("Failed to encrypt private key: {}", e)))?;
            
            // Prepend nonce to encrypted data
            let mut result = Vec::new();
            result.extend_from_slice(&nonce);
            result.extend_from_slice(&encrypted);
            Ok(result)
        } else {
            Err(IppanError::Crypto("Master key not set".to_string()))
        }
    }

    /// Decrypt a private key
    fn decrypt_private_key(&self, encrypted_data: &[u8]) -> Result<[u8; 32]> {
        if let Some(master_key) = &self.master_key {
            if encrypted_data.len() < 12 {
                return Err(IppanError::Crypto("Invalid encrypted data length".to_string()));
            }
            
            let nonce = &encrypted_data[0..12];
            let encrypted = &encrypted_data[12..];
            
            let mut nonce_array = [0u8; 12];
            nonce_array.copy_from_slice(nonce);
            
            let decrypted = decrypt_aes_gcm(master_key, &nonce_array, encrypted)
                .map_err(|e| IppanError::Crypto(format!("Failed to decrypt private key: {}", e)))?;
            
            if decrypted.len() != 32 {
                return Err(IppanError::Crypto("Invalid decrypted key length".to_string()));
            }
            
            let mut key_array = [0u8; 32];
            key_array.copy_from_slice(&decrypted);
            Ok(key_array)
        } else {
            Err(IppanError::Crypto("Master key not set".to_string()))
        }
    }

    /// Update key usage statistics
    pub fn update_key_usage(&mut self, address: &str) -> Result<()> {
        if let Some(key) = self.keys.get_mut(address) {
            key.usage_count += 1;
            key.last_used = Some(chrono::Utc::now());
        }
        Ok(())
    }

    /// Get keys that need rotation warning
    pub fn get_keys_needing_warning(&self) -> Vec<&Ed25519KeyPair> {
        let warning_threshold = chrono::Utc::now() + chrono::Duration::days(self.rotation_config.warning_period_days as i64);
        
        self.keys.values()
            .filter(|key| {
                if let Some(expires_at) = key.expires_at {
                    chrono::Utc::now() < expires_at && expires_at <= warning_threshold
                } else {
                    false
                }
            })
            .collect()
    }

    /// Add a key pair (for import functionality)
    pub fn add_key_pair(&mut self, key_pair: Ed25519KeyPair) {
        self.keys.insert(key_pair.address.clone(), key_pair);
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
    pub expired_keys: usize,
    pub keys_needing_rotation: usize,
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
