use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::password_hash::{rand_core::OsRng as ArgonOsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::{OsRng, RngCore};

use crate::errors::*;
use ippan_types::address::{decode_address, encode_address, ADDRESS_BYTES};

/// Generate a new Ed25519 key pair
pub fn generate_keypair() -> ([u8; 32], [u8; 32]) {
    let mut rng = OsRng;
    let mut secret = [0u8; 32];
    rng.fill_bytes(&mut secret);

    let signing_key = SigningKey::from_bytes(&secret);
    let verifying_key = signing_key.verifying_key();

    (secret, verifying_key.to_bytes())
}

/// Generate an IPPAN address from a public key
pub fn generate_address(public_key: &[u8; 32]) -> Result<String> {
    if public_key.len() != ADDRESS_BYTES {
        return Err(WalletError::InvalidAddress(format!(
            "Invalid public key length: {}",
            public_key.len()
        )));
    }

    Ok(encode_address(public_key))
}

/// Generate a new address with key pair
pub fn generate_new_address() -> Result<(String, [u8; 32], [u8; 32])> {
    let (private_key, public_key) = generate_keypair();
    let address = generate_address(&public_key)?;
    Ok((address, private_key, public_key))
}

/// Derive a key from a master seed using BIP32-like derivation
pub fn derive_key(master_seed: &[u8; 32], index: u64) -> ([u8; 32], [u8; 32]) {
    let mut hasher = blake3::Hasher::new();
    hasher.update(master_seed);
    hasher.update(&index.to_be_bytes());
    hasher.update(b"ippan_wallet_derivation");

    let derived_seed = hasher.finalize();
    let mut derived_bytes = [0u8; 32];
    derived_bytes.copy_from_slice(&derived_seed.as_bytes()[..32]);

    let signing_key = SigningKey::from_bytes(&derived_bytes);
    let verifying_key = signing_key.verifying_key();

    (derived_bytes, verifying_key.to_bytes())
}

/// Generate a master seed for HD wallets
pub fn generate_master_seed() -> [u8; 32] {
    let mut rng = OsRng;
    let mut seed = [0u8; 32];
    rng.fill_bytes(&mut seed);
    seed
}

/// Hash a password using Argon2
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut ArgonOsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| WalletError::CryptoError(format!("Password hashing failed: {}", e)))?;

    Ok(password_hash.to_string())
}

/// Verify a password against its hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| WalletError::CryptoError(format!("Invalid hash format: {}", e)))?;

    let argon2 = Argon2::default();
    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Encrypt data with AES-GCM
pub fn encrypt_data(data: &[u8], password: &str) -> Result<(String, String, String)> {
    // Derive key from password
    let key = derive_encryption_key(password)?;

    // Generate random nonce
    let mut rng = OsRng;
    let mut nonce_bytes = [0u8; 12];
    rng.fill_bytes(&mut nonce_bytes);

    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| WalletError::EncryptionError(format!("Cipher init failed: {}", e)))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| WalletError::EncryptionError(format!("Encryption failed: {}", e)))?;

    Ok((
        general_purpose::STANDARD.encode(&ciphertext),
        general_purpose::STANDARD.encode(&nonce_bytes),
        general_purpose::STANDARD.encode(&key),
    ))
}

/// Decrypt data with AES-GCM
pub fn decrypt_data(ciphertext: &str, nonce: &str, password: &str) -> Result<Vec<u8>> {
    let ciphertext_bytes = general_purpose::STANDARD
        .decode(ciphertext)
        .map_err(|e| WalletError::DecryptionError(format!("Invalid ciphertext: {}", e)))?;

    let nonce_bytes = general_purpose::STANDARD
        .decode(nonce)
        .map_err(|e| WalletError::DecryptionError(format!("Invalid nonce: {}", e)))?;

    let key = derive_encryption_key(password)?;

    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| WalletError::DecryptionError(format!("Cipher init failed: {}", e)))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext_bytes.as_ref())
        .map_err(|e| WalletError::DecryptionError(format!("Decryption failed: {}", e)))
}

/// Derive encryption key from password
fn derive_encryption_key(password: &str) -> Result<[u8; 32]> {
    let salt = b"ippan_wallet_salt_2024";
    let argon2 = Argon2::default();

    let mut key = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| WalletError::CryptoError(format!("Key derivation failed: {}", e)))?;

    Ok(key)
}

/// Sign a message with a private key
pub fn sign_message(message: &[u8], private_key: &[u8; 32]) -> Result<[u8; 64]> {
    let signing_key = SigningKey::from_bytes(private_key);
    let signature = signing_key.sign(message);
    Ok(signature.to_bytes())
}

/// Verify a signature
pub fn verify_signature(
    message: &[u8],
    signature: &[u8; 64],
    public_key: &[u8; 32],
) -> Result<bool> {
    let verifying_key = VerifyingKey::from_bytes(public_key)
        .map_err(|e| WalletError::CryptoError(format!("Invalid public key: {}", e)))?;

    let signature = Signature::from_slice(signature)
        .map_err(|e| WalletError::CryptoError(format!("Invalid signature: {}", e)))?;

    Ok(verifying_key.verify(message, &signature).is_ok())
}

/// Validate an IPPAN address
pub fn validate_address(address: &str) -> bool {
    decode_address(address).is_ok()
}

/// Generate a mnemonic seed phrase (simplified implementation)
pub fn generate_mnemonic() -> String {
    // This is a simplified implementation
    // In production, use a proper BIP39 implementation
    let words = [
        "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract", "absurd",
        "abuse", "access", "accident", "account", "accuse", "achieve", "acid", "acoustic",
        "acquire", "across", "act", "action", "actor", "actress", "actual", "adapt", "add",
        "addict", "address", "adjust", "admit", "adult", "advance", "advice", "aerobic", "affair",
        "afford", "afraid", "again", "age", "agent", "agree", "ahead", "aim", "air", "airport",
        "aisle", "alarm", "album", "alcohol", "alert", "alien", "all", "alley", "allow", "almost",
        "alone", "alpha", "already", "also", "alter",
    ];

    let mut rng = OsRng;
    let mut mnemonic = Vec::new();

    for _ in 0..12 {
        let index = (rng.next_u32() % words.len() as u32) as usize;
        mnemonic.push(words[index]);
    }

    mnemonic.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let (private_key, public_key) = generate_keypair();
        assert_eq!(private_key.len(), 32);
        assert_eq!(public_key.len(), 32);
    }

    #[test]
    fn test_address_generation() {
        let (address, private_key, public_key) = generate_new_address().unwrap();
        assert!(address.starts_with('i'));
        assert_eq!(address.len(), 65);
        assert_eq!(private_key.len(), 32);
        assert_eq!(public_key.len(), 32);
    }

    #[test]
    fn test_encryption_decryption() {
        let data = b"test data";
        let password = "test_password";

        let (ciphertext, nonce, _) = encrypt_data(data, password).unwrap();
        let decrypted = decrypt_data(&ciphertext, &nonce, password).unwrap();

        assert_eq!(data, decrypted.as_slice());
    }

    #[test]
    fn test_signature_verification() {
        let (private_key, public_key) = generate_keypair();
        let message = b"test message";

        let signature = sign_message(message, &private_key).unwrap();
        let is_valid = verify_signature(message, &signature, &public_key).unwrap();

        assert!(is_valid);
    }

    #[test]
    fn test_address_validation() {
        let (address, _, _) = generate_new_address().unwrap();
        assert!(validate_address(&address));
        assert!(!validate_address("invalid_address"));
    }
}
