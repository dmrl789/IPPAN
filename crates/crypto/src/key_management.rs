//! Key management for IPPAN
//!
//! Provides secure key generation, storage, and management
//! for cryptographic operations.

use anyhow::{anyhow, Result};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Key derivation functions
pub struct KeyDerivation;

impl KeyDerivation {
    /// Derive key from master key using HKDF
    pub fn hkdf_derive(master_key: &[u8], salt: &[u8], info: &[u8], length: usize) -> Result<Vec<u8>> {
        use hkdf::Hkdf;
        use sha2::Sha256;

        let hk = Hkdf::<Sha256>::new(Some(salt), master_key);
        let mut okm = vec![0u8; length];
        hk.expand(info, &mut okm)
            .map_err(|_| anyhow!("HKDF key derivation failed"))?;
        Ok(okm)
    }

    /// Derive key from password using scrypt
    pub fn scrypt_derive(password: &[u8], salt: &[u8], log_n: u8, r: u32, p: u32) -> Result<[u8; 32]> {
        use scrypt::{scrypt, Params};

        let params = Params::new(log_n, r, p, 32)
            .map_err(|_| anyhow!("Invalid scrypt parameters"))?;
        
        let mut key = [0u8; 32];
        scrypt(password, salt, &params, &mut key)
            .map_err(|_| anyhow!("Scrypt key derivation failed"))?;
        Ok(key)
    }

    /// Generate a random salt
    pub fn generate_salt() -> [u8; 32] {
        let mut salt = [0u8; 32];
        OsRng.fill_bytes(&mut salt);
        salt
    }
}

/// Key store for managing cryptographic keys
pub struct KeyStore {
    keys: Arc<RwLock<HashMap<String, StoredKey>>>,
    master_key: Option<[u8; 32]>,
}

/// Stored key information
#[derive(Debug, Clone)]
pub struct StoredKey {
    pub key_id: String,
    pub key_type: KeyType,
    pub encrypted_key: Vec<u8>,
    pub created_at: Instant,
    pub last_used: Instant,
    pub usage_count: u64,
    pub metadata: HashMap<String, String>,
}

/// Key types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum KeyType {
    Ed25519,
    AES256,
    ChaCha20,
    ECDH,
    HMAC,
}

impl KeyStore {
    /// Create a new key store
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            master_key: None,
        }
    }

    /// Initialize with master key
    pub fn initialize(&mut self, master_key: [u8; 32]) {
        self.master_key = Some(master_key);
    }

    /// Generate and store a new key
    pub fn generate_key(&self, key_id: String, key_type: KeyType) -> Result<()> {
        let key_data = self.generate_key_data(&key_type)?;
        self.store_key(key_id, key_type, key_data)?;
        Ok(())
    }

    /// Store a key
    pub fn store_key(&self, key_id: String, key_type: KeyType, key_data: Vec<u8>) -> Result<()> {
        let master_key = self.master_key
            .ok_or_else(|| anyhow!("Key store not initialized"))?;

        let encrypted_key = self.encrypt_key(&key_data, &master_key)?;
        
        let stored_key = StoredKey {
            key_id: key_id.clone(),
            key_type,
            encrypted_key,
            created_at: Instant::now(),
            last_used: Instant::now(),
            usage_count: 0,
            metadata: HashMap::new(),
        };

        let mut keys = self.keys.write().unwrap();
        keys.insert(key_id, stored_key);
        Ok(())
    }

    /// Retrieve a key
    pub fn get_key(&self, key_id: &str) -> Result<Vec<u8>> {
        let master_key = self.master_key
            .ok_or_else(|| anyhow!("Key store not initialized"))?;

        let mut keys = self.keys.write().unwrap();
        let stored_key = keys.get_mut(key_id)
            .ok_or_else(|| anyhow!("Key not found: {}", key_id))?;

        stored_key.last_used = Instant::now();
        stored_key.usage_count += 1;

        self.decrypt_key(&stored_key.encrypted_key, &master_key)
    }

    /// Delete a key
    pub fn delete_key(&self, key_id: &str) -> Result<()> {
        let mut keys = self.keys.write().unwrap();
        keys.remove(key_id)
            .ok_or_else(|| anyhow!("Key not found: {}", key_id))?;
        Ok(())
    }

    /// List all key IDs
    pub fn list_keys(&self) -> Vec<String> {
        let keys = self.keys.read().unwrap();
        keys.keys().cloned().collect()
    }

    /// Get key metadata
    pub fn get_key_metadata(&self, key_id: &str) -> Result<HashMap<String, String>> {
        let keys = self.keys.read().unwrap();
        let stored_key = keys.get(key_id)
            .ok_or_else(|| anyhow!("Key not found: {}", key_id))?;
        Ok(stored_key.metadata.clone())
    }

    /// Update key metadata
    pub fn update_key_metadata(&self, key_id: &str, metadata: HashMap<String, String>) -> Result<()> {
        let mut keys = self.keys.write().unwrap();
        let stored_key = keys.get_mut(key_id)
            .ok_or_else(|| anyhow!("Key not found: {}", key_id))?;
        stored_key.metadata = metadata;
        Ok(())
    }

    /// Generate key data based on type
    fn generate_key_data(&self, key_type: &KeyType) -> Result<Vec<u8>> {
        match key_type {
            KeyType::Ed25519 => {
                let mut key = [0u8; 32];
                OsRng.fill_bytes(&mut key);
                Ok(key.to_vec())
            }
            KeyType::AES256 => {
                let mut key = [0u8; 32];
                OsRng.fill_bytes(&mut key);
                Ok(key.to_vec())
            }
            KeyType::ChaCha20 => {
                let mut key = [0u8; 32];
                OsRng.fill_bytes(&mut key);
                Ok(key.to_vec())
            }
            KeyType::ECDH => {
                let mut key = [0u8; 32];
                OsRng.fill_bytes(&mut key);
                Ok(key.to_vec())
            }
            KeyType::HMAC => {
                let mut key = [0u8; 64];
                OsRng.fill_bytes(&mut key);
                Ok(key.to_vec())
            }
        }
    }

    /// Encrypt key for storage
    fn encrypt_key(&self, key_data: &[u8], master_key: &[u8; 32]) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, Key, Nonce};
        use aes_gcm::aead::{Aead, KeyInit};

        let key = Key::from_slice(master_key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(&[0u8; 12]); // In production, use random nonce

        cipher.encrypt(nonce, key_data)
            .map_err(|_| anyhow!("Key encryption failed"))
    }

    /// Decrypt key from storage
    fn decrypt_key(&self, encrypted_key: &[u8], master_key: &[u8; 32]) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, Key, Nonce};
        use aes_gcm::aead::{Aead, KeyInit};

        let key = Key::from_slice(master_key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(&[0u8; 12]); // In production, use stored nonce

        cipher.decrypt(nonce, encrypted_key)
            .map_err(|_| anyhow!("Key decryption failed"))
    }
}

/// Key manager for high-level key operations
pub struct KeyManager {
    key_store: Arc<KeyStore>,
    key_cache: Arc<RwLock<HashMap<String, CachedKey>>>,
}

/// Cached key information
#[derive(Debug, Clone)]
struct CachedKey {
    key_data: Vec<u8>,
    key_type: KeyType,
    cached_at: Instant,
    ttl: Duration,
}

impl KeyManager {
    /// Create a new key manager
    pub fn new(key_store: Arc<KeyStore>) -> Self {
        Self {
            key_store,
            key_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate a new key
    pub fn generate_key(&self, key_id: String, key_type: KeyType) -> Result<()> {
        self.key_store.generate_key(key_id, key_type)
    }

    /// Get a key (with caching)
    pub fn get_key(&self, key_id: &str) -> Result<Vec<u8>> {
        // Check cache first
        {
            let cache = self.key_cache.read().unwrap();
            if let Some(cached_key) = cache.get(key_id) {
                if cached_key.cached_at.elapsed() < cached_key.ttl {
                    return Ok(cached_key.key_data.clone());
                }
            }
        }

        // Get from store and cache
        let key_data = self.key_store.get_key(key_id)?;
        let key_type = self.get_key_type(key_id)?;
        
        {
            let mut cache = self.key_cache.write().unwrap();
            cache.insert(key_id.to_string(), CachedKey {
                key_data: key_data.clone(),
                key_type,
                cached_at: Instant::now(),
                ttl: Duration::from_secs(300), // 5 minutes
            });
        }

        Ok(key_data)
    }

    /// Get key type
    fn get_key_type(&self, key_id: &str) -> Result<KeyType> {
        let keys = self.key_store.keys.read().unwrap();
        let stored_key = keys.get(key_id)
            .ok_or_else(|| anyhow!("Key not found: {}", key_id))?;
        Ok(stored_key.key_type.clone())
    }

    /// Rotate a key
    pub fn rotate_key(&self, key_id: &str) -> Result<()> {
        let key_type = self.get_key_type(key_id)?;
        let new_key_data = self.key_store.generate_key_data(&key_type)?;
        self.key_store.store_key(key_id.to_string(), key_type, new_key_data)?;
        
        // Remove from cache
        {
            let mut cache = self.key_cache.write().unwrap();
            cache.remove(key_id);
        }
        
        Ok(())
    }

    /// Clean up expired cache entries
    pub fn cleanup_cache(&self) {
        let mut cache = self.key_cache.write().unwrap();
        let now = Instant::now();
        cache.retain(|_, cached_key| {
            now.duration_since(cached_key.cached_at) < cached_key.ttl
        });
    }

    /// Get key statistics
    pub fn get_key_stats(&self) -> KeyStats {
        let keys = self.key_store.keys.read().unwrap();
        let cache = self.key_cache.read().unwrap();
        
        let total_keys = keys.len();
        let cached_keys = cache.len();
        let total_usage: u64 = keys.values().map(|k| k.usage_count).sum();
        
        KeyStats {
            total_keys,
            cached_keys,
            total_usage,
        }
    }
}

/// Key statistics
#[derive(Debug, Clone)]
pub struct KeyStats {
    pub total_keys: usize,
    pub cached_keys: usize,
    pub total_usage: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_derivation() {
        let master_key = b"test_master_key";
        let salt = KeyDerivation::generate_salt();
        let info = b"test_info";
        let length = 32;

        let derived_key = KeyDerivation::hkdf_derive(master_key, &salt, info, length).unwrap();
        assert_eq!(derived_key.len(), length);
    }

    #[test]
    fn test_key_store() {
        let mut key_store = KeyStore::new();
        let master_key = [1u8; 32];
        key_store.initialize(master_key);

        let key_id = "test_key".to_string();
        let key_type = KeyType::Ed25519;
        
        key_store.generate_key(key_id.clone(), key_type.clone()).unwrap();
        let retrieved_key = key_store.get_key(&key_id).unwrap();
        assert!(!retrieved_key.is_empty());
    }

    #[test]
    fn test_key_manager() {
        let key_store = Arc::new(KeyStore::new());
        let mut key_store_mut = key_store.as_ref();
        let master_key = [1u8; 32];
        key_store_mut.initialize(master_key);

        let key_manager = KeyManager::new(key_store);
        let key_id = "test_key".to_string();
        let key_type = KeyType::AES256;
        
        key_manager.generate_key(key_id.clone(), key_type).unwrap();
        let retrieved_key = key_manager.get_key(&key_id).unwrap();
        assert!(!retrieved_key.is_empty());
    }
}
