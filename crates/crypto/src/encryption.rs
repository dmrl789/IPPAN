//! Encryption algorithms for IPPAN
//!
//! Provides symmetric and asymmetric encryption capabilities
//! for secure data transmission and storage.

use anyhow::{anyhow, Result};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Encryption error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionError {
    InvalidKey,
    InvalidNonce,
    InvalidCiphertext,
    DecryptionFailed,
    KeyGenerationFailed,
    InvalidAlgorithm,
}

impl std::fmt::Display for EncryptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptionError::InvalidKey => write!(f, "Invalid encryption key"),
            EncryptionError::InvalidNonce => write!(f, "Invalid nonce"),
            EncryptionError::InvalidCiphertext => write!(f, "Invalid ciphertext"),
            EncryptionError::DecryptionFailed => write!(f, "Decryption failed"),
            EncryptionError::KeyGenerationFailed => write!(f, "Key generation failed"),
            EncryptionError::InvalidAlgorithm => write!(f, "Invalid encryption algorithm"),
        }
    }
}

impl std::error::Error for EncryptionError {}

/// AES-256-GCM encryption implementation
pub struct AES256GCM {
    key: [u8; 32],
}

impl AES256GCM {
    /// Create a new AES-256-GCM instance
    pub fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    /// Generate a new random key
    pub fn generate_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        key
    }

    /// Encrypt data
    pub fn encrypt(&self, plaintext: &[u8], nonce: &[u8; 12]) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, Key, Nonce};
        use aes_gcm::aead::{Aead, KeyInit};

        let key = Key::<Aes256Gcm>::from_slice(&self.key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::<Aes256Gcm>::from_slice(nonce);

        cipher.encrypt(nonce, plaintext)
            .map_err(|_| anyhow!("Encryption failed"))
    }

    /// Decrypt data
    pub fn decrypt(&self, ciphertext: &[u8], nonce: &[u8; 12]) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, Key, Nonce};
        use aes_gcm::aead::{Aead, KeyInit};

        let key = Key::<Aes256Gcm>::from_slice(&self.key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::<Aes256Gcm>::from_slice(nonce);

        cipher.decrypt(nonce, ciphertext)
            .map_err(|_| anyhow!("Decryption failed"))
    }

    /// Generate a random nonce
    pub fn generate_nonce() -> [u8; 12] {
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);
        nonce
    }
}

/// ChaCha20-Poly1305 encryption implementation
pub struct ChaCha20Poly1305 {
    key: [u8; 32],
}

impl ChaCha20Poly1305 {
    /// Create a new ChaCha20-Poly1305 instance
    pub fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    /// Generate a new random key
    pub fn generate_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        key
    }

    /// Encrypt data
    pub fn encrypt(&self, plaintext: &[u8], nonce: &[u8; 12]) -> Result<Vec<u8>> {
        use chacha20poly1305::{ChaCha20Poly1305 as Cipher, Key, Nonce};
        use chacha20poly1305::aead::{Aead, KeyInit};

        let key = Key::<Cipher>::from_slice(&self.key);
        let cipher = Cipher::new(key);
        let nonce = Nonce::<Cipher>::from_slice(nonce);

        cipher.encrypt(nonce, plaintext)
            .map_err(|_| anyhow!("Encryption failed"))
    }

    /// Decrypt data
    pub fn decrypt(&self, ciphertext: &[u8], nonce: &[u8; 12]) -> Result<Vec<u8>> {
        use chacha20poly1305::{ChaCha20Poly1305 as Cipher, Key, Nonce};
        use chacha20poly1305::aead::{Aead, KeyInit};

        let key = Key::<Cipher>::from_slice(&self.key);
        let cipher = Cipher::new(key);
        let nonce = Nonce::<Cipher>::from_slice(nonce);

        cipher.decrypt(nonce, ciphertext)
            .map_err(|_| anyhow!("Decryption failed"))
    }

    /// Generate a random nonce
    pub fn generate_nonce() -> [u8; 12] {
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);
        nonce
    }
}

/// Hybrid encryption using ECIES
pub struct ECIES {
    private_key: [u8; 32],
    public_key: [u8; 32],
}

impl ECIES {
    /// Create a new ECIES instance
    pub fn new(private_key: [u8; 32], public_key: [u8; 32]) -> Self {
        Self {
            private_key,
            public_key,
        }
    }

    /// Generate a new key pair
    pub fn generate_keypair() -> ([u8; 32], [u8; 32]) {
        let mut private_key = [0u8; 32];
        OsRng.fill_bytes(&mut private_key);
        
        // In a real implementation, this would derive the public key
        // from the private key using elliptic curve operations
        let mut public_key = [0u8; 32];
        OsRng.fill_bytes(&mut public_key);
        
        (private_key, public_key)
    }

    /// Encrypt data for a specific public key
    pub fn encrypt(&self, plaintext: &[u8], recipient_public_key: &[u8; 32]) -> Result<Vec<u8>> {
        // Generate ephemeral key pair
        let (ephemeral_private, ephemeral_public) = Self::generate_keypair();
        
        // Derive shared secret (simplified)
        let shared_secret = self.derive_shared_secret(&ephemeral_private, recipient_public_key)?;
        
        // Use AES-256-GCM for symmetric encryption
        let aes = AES256GCM::new(shared_secret);
        let nonce = AES256GCM::generate_nonce();
        let ciphertext = aes.encrypt(plaintext, &nonce)?;
        
        // Combine ephemeral public key, nonce, and ciphertext
        let mut result = Vec::new();
        result.extend_from_slice(&ephemeral_public);
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }

    /// Decrypt data using private key
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() < 64 { // 32 (ephemeral pub) + 12 (nonce) + 16 (min ciphertext)
            return Err(anyhow!("Invalid ciphertext length"));
        }

        let ephemeral_public = &ciphertext[0..32];
        let nonce = &ciphertext[32..44];
        let encrypted_data = &ciphertext[44..];

        // Derive shared secret
        let shared_secret = self.derive_shared_secret(&self.private_key, &ephemeral_public.try_into().unwrap())?;
        
        // Use AES-256-GCM for symmetric decryption
        let aes = AES256GCM::new(shared_secret);
        let nonce_array: [u8; 12] = nonce.try_into().unwrap();
        aes.decrypt(encrypted_data, &nonce_array)
    }

    /// Derive shared secret (simplified implementation)
    fn derive_shared_secret(&self, private_key: &[u8; 32], public_key: &[u8; 32]) -> Result<[u8; 32]> {
        // In a real implementation, this would use ECDH
        // For now, we'll use a simple XOR operation
        let mut shared_secret = [0u8; 32];
        for i in 0..32 {
            shared_secret[i] = private_key[i] ^ public_key[i];
        }
        Ok(shared_secret)
    }
}

/// Password-based key derivation
pub struct PBKDF2;

impl PBKDF2 {
    /// Derive key from password using PBKDF2
    pub fn derive_key(password: &[u8], salt: &[u8], iterations: u32) -> Result<[u8; 32]> {
        use pbkdf2::pbkdf2;
        use sha2::Sha256;

        let mut key = [0u8; 32];
        pbkdf2::<sha2::Sha256>(password, salt, iterations, &mut key)
            .map_err(|_| anyhow!("Key derivation failed"))?;
        Ok(key)
    }

    /// Generate a random salt
    pub fn generate_salt() -> [u8; 32] {
        let mut salt = [0u8; 32];
        OsRng.fill_bytes(&mut salt);
        salt
    }
}

/// Argon2 password hashing
pub struct Argon2;

impl Argon2 {
    /// Hash password using Argon2
    pub fn hash_password(password: &[u8]) -> Result<String> {
        use argon2::Argon2;
        use argon2::password_hash::{PasswordHash, PasswordHasher, SaltString};

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        let password_hash = argon2
            .hash_password(password, &salt)
            .map_err(|_| anyhow!("Password hashing failed"))?;
        
        Ok(password_hash.to_string())
    }

    /// Verify password against hash
    pub fn verify_password(password: &[u8], hash: &str) -> Result<bool> {
        use argon2::Argon2;
        use argon2::password_hash::{PasswordHash, PasswordVerifier};

        let parsed_hash = PasswordHash::new(hash)
            .map_err(|_| anyhow!("Invalid hash format"))?;
        
        let argon2 = Argon2::default();
        Ok(argon2.verify_password(password, &parsed_hash).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes256gcm_encryption() {
        let key = AES256GCM::generate_key();
        let aes = AES256GCM::new(key);
        let nonce = AES256GCM::generate_nonce();
        let plaintext = b"Hello, IPPAN!";

        let ciphertext = aes.encrypt(plaintext, &nonce).unwrap();
        let decrypted = aes.decrypt(&ciphertext, &nonce).unwrap();

        assert_eq!(plaintext, &decrypted[..]);
    }

    #[test]
    fn test_chacha20poly1305_encryption() {
        let key = ChaCha20Poly1305::generate_key();
        let chacha = ChaCha20Poly1305::new(key);
        let nonce = ChaCha20Poly1305::generate_nonce();
        let plaintext = b"Hello, IPPAN!";

        let ciphertext = chacha.encrypt(plaintext, &nonce).unwrap();
        let decrypted = chacha.decrypt(&ciphertext, &nonce).unwrap();

        assert_eq!(plaintext, &decrypted[..]);
    }

    #[test]
    fn test_ecies_encryption() {
        let (private_key, public_key) = ECIES::generate_keypair();
        let ecies = ECIES::new(private_key, public_key);
        let plaintext = b"Hello, IPPAN!";

        let ciphertext = ecies.encrypt(plaintext, &public_key).unwrap();
        let decrypted = ecies.decrypt(&ciphertext).unwrap();

        assert_eq!(plaintext, &decrypted[..]);
    }

    #[test]
    fn test_pbkdf2_key_derivation() {
        let password = b"test_password";
        let salt = PBKDF2::generate_salt();
        let iterations = 10000;

        let key1 = PBKDF2::derive_key(password, &salt, iterations).unwrap();
        let key2 = PBKDF2::derive_key(password, &salt, iterations).unwrap();

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_argon2_password_hashing() {
        let password = b"test_password";
        let hash = Argon2::hash_password(password).unwrap();
        let is_valid = Argon2::verify_password(password, &hash).unwrap();

        assert!(is_valid);
    }
}
