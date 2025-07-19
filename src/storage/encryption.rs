//! Encryption for IPPAN storage

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit};
use aes_gcm::aead::Aead;
use rand::RngCore;
use sha2::{Sha256, Digest};

/// Encryption key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionKey {
    /// Key ID
    pub key_id: String,
    /// Key data (encrypted)
    pub key_data: Vec<u8>,
    /// Key algorithm
    pub algorithm: EncryptionAlgorithm,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Expires at timestamp
    pub expires_at: Option<DateTime<Utc>>,
    /// Key status
    pub status: KeyStatus,
}

/// Encryption algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    /// AES-256-GCM
    Aes256Gcm,
    /// ChaCha20-Poly1305
    ChaCha20Poly1305,
}

/// Key status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyStatus {
    /// Key is active
    Active,
    /// Key is inactive
    Inactive,
    /// Key is expired
    Expired,
    /// Key is revoked
    Revoked,
}

/// Encrypted data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Key ID used for encryption
    pub key_id: String,
    /// Nonce used for encryption
    pub nonce: Vec<u8>,
    /// Encrypted data
    pub data: Vec<u8>,
    /// Authentication tag
    pub tag: Vec<u8>,
    /// Encryption timestamp
    pub encrypted_at: DateTime<Utc>,
}

/// Encryption manager
pub struct EncryptionManager {
    /// Encryption keys
    keys: Arc<RwLock<HashMap<String, EncryptionKey>>>,
    /// Master key (for key encryption)
    master_key: Option<Vec<u8>>,
    /// Key rotation interval (days)
    key_rotation_interval: u32,
    /// Running flag
    running: bool,
}

impl EncryptionManager {
    /// Create a new encryption manager
    pub fn new(key_rotation_interval: u32) -> Result<Self> {
        Ok(Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            master_key: None,
            key_rotation_interval,
            running: false,
        })
    }

    /// Start the encryption manager
    pub async fn start(&mut self) -> Result<()> {
        log::info!("Starting encryption manager");
        self.running = true;
        
        // Generate master key if not exists
        if self.master_key.is_none() {
            self.generate_master_key()?;
        }
        
        // Start key rotation task
        let keys = self.keys.clone();
        let rotation_interval = self.key_rotation_interval;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(rotation_interval as u64 * 24 * 60 * 60)
            );
            
            loop {
                interval.tick().await;
                Self::rotate_keys(&keys).await;
            }
        });
        
        Ok(())
    }

    /// Stop the encryption manager
    pub async fn stop(&mut self) -> Result<()> {
        log::info!("Stopping encryption manager");
        self.running = false;
        Ok(())
    }

    /// Generate a new encryption key
    pub async fn generate_key(&self, key_id: &str, algorithm: EncryptionAlgorithm) -> Result<()> {
        let key_data = match algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                let mut key = vec![0u8; 32];
                rand::thread_rng().fill_bytes(&mut key);
                key
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                let mut key = vec![0u8; 32];
                rand::thread_rng().fill_bytes(&mut key);
                key
            }
        };
        
        let encrypted_key = self.encrypt_key_data(&key_data)?;
        
        let key = EncryptionKey {
            key_id: key_id.to_string(),
            key_data: encrypted_key,
            algorithm,
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::days(self.key_rotation_interval as i64)),
            status: KeyStatus::Active,
        };
        
        let mut keys = self.keys.write().await;
        keys.insert(key_id.to_string(), key);
        
        log::info!("Generated encryption key: {}", key_id);
        Ok(())
    }

    /// Encrypt data
    pub async fn encrypt_data(&self, data: &[u8], key_id: &str) -> Result<EncryptedData> {
        let keys = self.keys.read().await;
        
        let key = keys.get(key_id).ok_or_else(|| {
            crate::error::IppanError::Storage(
                format!("Encryption key not found: {}", key_id)
            )
        })?;
        
        if key.status != KeyStatus::Active {
            return Err(crate::error::IppanError::Storage(
                format!("Encryption key is not active: {}", key_id)
            ));
        }
        
        let decrypted_key = self.decrypt_key_data(&key.key_data)?;
        
        let encrypted_data = match key.algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                self.encrypt_aes256gcm(data, &decrypted_key, key_id)?
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                self.encrypt_chacha20poly1305(data, &decrypted_key, key_id)?
            }
        };
        
        Ok(encrypted_data)
    }

    /// Decrypt data
    pub async fn decrypt_data(&self, encrypted_data: &EncryptedData) -> Result<Vec<u8>> {
        let keys = self.keys.read().await;
        
        let key = keys.get(&encrypted_data.key_id).ok_or_else(|| {
            crate::error::IppanError::Storage(
                format!("Encryption key not found: {}", encrypted_data.key_id)
            )
        })?;
        
        if key.status != KeyStatus::Active {
            return Err(crate::error::IppanError::Storage(
                format!("Encryption key is not active: {}", encrypted_data.key_id)
            ));
        }
        
        let decrypted_key = self.decrypt_key_data(&key.key_data)?;
        
        let decrypted_data = match key.algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                self.decrypt_aes256gcm(encrypted_data, &decrypted_key)?
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                self.decrypt_chacha20poly1305(encrypted_data, &decrypted_key)?
            }
        };
        
        Ok(decrypted_data)
    }

    /// Revoke a key
    pub async fn revoke_key(&self, key_id: &str) -> Result<()> {
        let mut keys = self.keys.write().await;
        
        if let Some(key) = keys.get_mut(key_id) {
            key.status = KeyStatus::Revoked;
            log::info!("Revoked encryption key: {}", key_id);
        }
        
        Ok(())
    }

    /// Get encryption statistics
    pub async fn get_encryption_stats(&self) -> EncryptionStats {
        let keys = self.keys.read().await;
        
        let total_keys = keys.len();
        let active_keys = keys.values()
            .filter(|key| key.status == KeyStatus::Active)
            .count();
        
        let expired_keys = keys.values()
            .filter(|key| key.status == KeyStatus::Expired)
            .count();
        
        let revoked_keys = keys.values()
            .filter(|key| key.status == KeyStatus::Revoked)
            .count();
        
        EncryptionStats {
            total_keys,
            active_keys,
            expired_keys,
            revoked_keys,
            key_rotation_interval: self.key_rotation_interval,
        }
    }

    /// Generate master key
    fn generate_master_key(&mut self) -> Result<()> {
        let mut master_key = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut master_key);
        self.master_key = Some(master_key);
        Ok(())
    }

    /// Encrypt key data with master key
    fn encrypt_key_data(&self, key_data: &[u8]) -> Result<Vec<u8>> {
        let master_key = self.master_key.as_ref().ok_or_else(|| {
            crate::error::IppanError::Storage(
                "Master key not available".to_string()
            )
        })?;
        
        // Simple XOR encryption for demonstration
        // In production, use proper key encryption
        let mut encrypted = Vec::new();
        for (i, &byte) in key_data.iter().enumerate() {
            encrypted.push(byte ^ master_key[i % master_key.len()]);
        }
        
        Ok(encrypted)
    }

    /// Decrypt key data with master key
    fn decrypt_key_data(&self, encrypted_key_data: &[u8]) -> Result<Vec<u8>> {
        let master_key = self.master_key.as_ref().ok_or_else(|| {
            crate::error::IppanError::Storage(
                "Master key not available".to_string()
            )
        })?;
        
        // Simple XOR decryption for demonstration
        let mut decrypted = Vec::new();
        for (i, &byte) in encrypted_key_data.iter().enumerate() {
            decrypted.push(byte ^ master_key[i % master_key.len()]);
        }
        
        Ok(decrypted)
    }

    /// Encrypt data with AES-256-GCM
    fn encrypt_aes256gcm(&self, data: &[u8], key: &[u8], key_id: &str) -> Result<EncryptedData> {
        // Ensure key is exactly 32 bytes for AES-256
        if key.len() != 32 {
            return Err(crate::error::IppanError::Storage(
                format!("Invalid key size: {} (expected 32)", key.len())
            ));
        }
        
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| crate::error::IppanError::Storage(format!("Invalid key: {}", e)))?;
        
        // Generate 12-byte nonce for AES-GCM
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher.encrypt(nonce, data)
            .map_err(|e| crate::error::IppanError::Storage(format!("Encryption failed: {}", e)))?;
        
        // Split ciphertext into data and tag
        let tag_size = 16;
        if ciphertext.len() < tag_size {
            return Err(crate::error::IppanError::Storage(
                "Invalid ciphertext length".to_string()
            ));
        }
        
        let data_len = ciphertext.len() - tag_size;
        let encrypted_data = ciphertext[..data_len].to_vec();
        let tag = ciphertext[data_len..].to_vec();
        
        Ok(EncryptedData {
            key_id: key_id.to_string(),
            nonce: nonce_bytes.to_vec(),
            data: encrypted_data,
            tag,
            encrypted_at: Utc::now(),
        })
    }

    /// Decrypt data with AES-256-GCM
    fn decrypt_aes256gcm(&self, encrypted_data: &EncryptedData, key: &[u8]) -> Result<Vec<u8>> {
        // Ensure key is exactly 32 bytes for AES-256
        if key.len() != 32 {
            return Err(crate::error::IppanError::Storage(
                format!("Invalid key size: {} (expected 32)", key.len())
            ));
        }
        
        // Ensure nonce is exactly 12 bytes
        if encrypted_data.nonce.len() != 12 {
            return Err(crate::error::IppanError::Storage(
                format!("Invalid nonce size: {} (expected 12)", encrypted_data.nonce.len())
            ));
        }
        
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| crate::error::IppanError::Storage(format!("Invalid key: {}", e)))?;
        
        let nonce = Nonce::from_slice(&encrypted_data.nonce);
        
        // Combine data and tag
        let mut ciphertext = encrypted_data.data.clone();
        ciphertext.extend_from_slice(&encrypted_data.tag);
        
        let plaintext = cipher.decrypt(nonce, ciphertext.as_slice())
            .map_err(|e| crate::error::IppanError::Storage(format!("Decryption failed: {}", e)))?;
        
        Ok(plaintext)
    }

    /// Encrypt data with ChaCha20-Poly1305
    fn encrypt_chacha20poly1305(&self, _data: &[u8], _key: &[u8], _key_id: &str) -> Result<EncryptedData> {
        // TODO: Implement ChaCha20-Poly1305 encryption
        // For now, return a placeholder
        Err(crate::error::IppanError::Storage(
            "ChaCha20-Poly1305 not implemented yet".to_string()
        ))
    }

    /// Decrypt data with ChaCha20-Poly1305
    fn decrypt_chacha20poly1305(&self, _encrypted_data: &EncryptedData, _key: &[u8]) -> Result<Vec<u8>> {
        // TODO: Implement ChaCha20-Poly1305 decryption
        // For now, return a placeholder
        Err(crate::error::IppanError::Storage(
            "ChaCha20-Poly1305 not implemented yet".to_string()
        ))
    }

    /// Rotate encryption keys
    async fn rotate_keys(keys: &Arc<RwLock<HashMap<String, EncryptionKey>>>) {
        let mut keys = keys.write().await;
        let now = Utc::now();
        
        for key in keys.values_mut() {
            if let Some(expires_at) = key.expires_at {
                if now >= expires_at {
                    key.status = KeyStatus::Expired;
                    log::info!("Key expired: {}", key.key_id);
                }
            }
        }
    }
}

/// Encryption statistics
#[derive(Debug, Serialize)]
pub struct EncryptionStats {
    pub total_keys: usize,
    pub active_keys: usize,
    pub expired_keys: usize,
    pub revoked_keys: usize,
    pub key_rotation_interval: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encryption_manager_creation() {
        let manager = EncryptionManager::new(30).unwrap();
        
        assert_eq!(manager.key_rotation_interval, 30);
        assert!(!manager.running);
    }

    #[tokio::test]
    async fn test_encryption_manager_start_stop() {
        let mut manager = EncryptionManager::new(30).unwrap();
        
        manager.start().await.unwrap();
        assert!(manager.running);
        
        manager.stop().await.unwrap();
        assert!(!manager.running);
    }

    #[tokio::test]
    async fn test_key_generation() {
        let mut manager = EncryptionManager::new(30).unwrap();
        manager.start().await.unwrap();
        
        manager.generate_key("test_key", EncryptionAlgorithm::Aes256Gcm).await.unwrap();
        
        let stats = manager.get_encryption_stats().await;
        assert_eq!(stats.total_keys, 1);
        assert_eq!(stats.active_keys, 1);
    }

    #[tokio::test]
    async fn test_data_encryption_decryption() {
        let mut manager = EncryptionManager::new(30).unwrap();
        manager.start().await.unwrap();
        
        // Generate a key
        manager.generate_key("test_key", EncryptionAlgorithm::Aes256Gcm).await.unwrap();
        
        // Encrypt data
        let data = b"Hello, World!";
        let encrypted = manager.encrypt_data(data, "test_key").await.unwrap();
        
        // Decrypt data
        let decrypted = manager.decrypt_data(&encrypted).await.unwrap();
        
        assert_eq!(decrypted, data);
    }

    #[tokio::test]
    async fn test_key_revocation() {
        let mut manager = EncryptionManager::new(30).unwrap();
        manager.start().await.unwrap();
        
        manager.generate_key("test_key", EncryptionAlgorithm::Aes256Gcm).await.unwrap();
        manager.revoke_key("test_key").await.unwrap();
        
        let stats = manager.get_encryption_stats().await;
        assert_eq!(stats.revoked_keys, 1);
    }
}
