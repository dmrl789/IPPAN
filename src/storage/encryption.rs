use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit};
use aes_gcm::aead::{Aead, NewAead};
use rand::{Rng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use crate::Result;

/// AES-256-GCM encryption manager
pub struct EncryptionManager {
    /// Master encryption key
    key: [u8; 32],
}

/// Encrypted data with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Encrypted data
    pub data: Vec<u8>,
    /// Nonce used for encryption
    pub nonce: [u8; 12],
    /// Authentication tag
    pub tag: [u8; 16],
}

impl EncryptionManager {
    /// Create a new encryption manager
    pub fn new(master_key: &[u8; 32]) -> Self {
        let key = Key::from_slice(master_key);
        Self {
            key: *master_key,
        }
    }

    /// Encrypt data
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new(Key::from_slice(&self.key));
        let nonce = self.generate_nonce();
        let nonce = Nonce::from_slice(&nonce);
        
        let ciphertext = cipher.encrypt(nonce, data)
            .map_err(|e| crate::IppanError::Storage(format!("Encryption failed: {}", e)))?;
        
        // Prepend nonce to ciphertext
        let mut result = Vec::new();
        result.extend_from_slice(&self.generate_nonce());
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }

    /// Decrypt data
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() < 12 {
            return Err(crate::IppanError::Storage("Invalid ciphertext length".to_string()));
        }
        
        let cipher = Aes256Gcm::new(Key::from_slice(&self.key));
        let nonce = Nonce::from_slice(&ciphertext[..12]);
        let data = &ciphertext[12..];
        
        let plaintext = cipher.decrypt(nonce, data)
            .map_err(|e| crate::IppanError::Storage(format!("Decryption failed: {}", e)))?;
        
        Ok(plaintext)
    }

    /// Generate a new random nonce
    fn generate_nonce(&self) -> [u8; 12] {
        let mut nonce = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce);
        nonce
    }

    /// Encrypt a file in chunks
    pub fn encrypt_file(&self, data: &[u8], chunk_size: usize) -> Result<Vec<EncryptedData>> {
        let mut encrypted_chunks = Vec::new();
        
        for chunk in data.chunks(chunk_size) {
            let encrypted = self.encrypt(chunk)?;
            encrypted_chunks.push(EncryptedData {
                data: encrypted,
                nonce: self.generate_nonce(),
                tag: [0u8; 16],
            });
        }
        
        Ok(encrypted_chunks)
    }

    /// Decrypt a file from chunks
    pub fn decrypt_file(&self, encrypted_chunks: &[EncryptedData]) -> Result<Vec<u8>> {
        let mut decrypted_data = Vec::new();
        
        for chunk in encrypted_chunks {
            let decrypted = self.decrypt(&chunk.data)?;
            decrypted_data.extend(decrypted);
        }
        
        Ok(decrypted_data)
    }

    /// Generate a deterministic encryption key from a file hash
    pub fn derive_file_key(&self, file_hash: &[u8; 32], salt: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.key);
        hasher.update(file_hash);
        hasher.update(salt);
        hasher.finalize().into()
    }

    /// Encrypt with a derived key
    pub fn encrypt_with_derived_key(&self, data: &[u8], file_hash: &[u8; 32], salt: &[u8]) -> Result<EncryptedData> {
        let derived_key = self.derive_file_key(file_hash, salt);
        let derived_encryption = EncryptionManager::new(&derived_key);
        derived_encryption.encrypt(data).map(|encrypted| EncryptedData {
            data: encrypted,
            nonce: self.generate_nonce(),
            tag: [0u8; 16],
        })
    }

    /// Decrypt with a derived key
    pub fn decrypt_with_derived_key(&self, encrypted_data: &EncryptedData, file_hash: &[u8; 32], salt: &[u8]) -> Result<Vec<u8>> {
        let derived_key = self.derive_file_key(file_hash, salt);
        let derived_encryption = EncryptionManager::new(&derived_key);
        derived_encryption.decrypt(&encrypted_data.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        let master_key = EncryptionManager::new(&[0u8; 32]).key;
        let encryption = EncryptionManager::new(&master_key);
        
        let original_data = b"Hello, IPPAN! This is a test message.";
        let encrypted = encryption.encrypt(original_data).unwrap();
        let decrypted = encryption.decrypt(&encrypted).unwrap();
        
        assert_eq!(original_data, decrypted.as_slice());
    }

    #[test]
    fn test_file_encryption() {
        let master_key = EncryptionManager::new(&[0u8; 32]).key;
        let encryption = EncryptionManager::new(&master_key);
        
        let original_data = b"This is a larger file that needs to be encrypted in chunks. It contains multiple sentences and should be split into smaller pieces for processing.";
        let encrypted_chunks = encryption.encrypt_file(original_data, 32).unwrap();
        let decrypted_data = encryption.decrypt_file(&encrypted_chunks).unwrap();
        
        assert_eq!(original_data, decrypted_data.as_slice());
    }

    #[test]
    fn test_derived_key_encryption() {
        let master_key = EncryptionManager::new(&[0u8; 32]).key;
        let encryption = EncryptionManager::new(&master_key);
        
        let file_hash = [1u8; 32];
        let salt = b"test_salt";
        let original_data = b"Data encrypted with derived key";
        
        let encrypted = encryption.encrypt_with_derived_key(original_data, &file_hash, salt).unwrap();
        let decrypted = encryption.decrypt_with_derived_key(&encrypted, &file_hash, salt).unwrap();
        
        assert_eq!(original_data, decrypted.as_slice());
    }
}
