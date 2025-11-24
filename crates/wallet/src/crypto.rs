use aes_gcm::aead::{Aead, AeadCore, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::password_hash::{rand_core::OsRng as ArgonOsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::{OsRng, RngCore};
#[cfg(test)]
use std::cell::RefCell;
use std::env;

use crate::errors::*;
use ippan_types::address::{decode_address, encode_address, ADDRESS_BYTES};

const LEGACY_SALT_ENV: &str = "IPPAN_WALLET_LEGACY_SALT_B64";

#[cfg(test)]
const LEGACY_SALT_FALLBACK: &[u8] = b"ippan_wallet_salt_2024";

#[cfg(test)]
thread_local! {
    static LEGACY_SALT_OVERRIDE: RefCell<Option<Vec<u8>>> = RefCell::new(None);
}

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
    // Generate per-entry salt to avoid hard-coded cryptographic material
    let mut rng = OsRng;
    let mut salt = [0u8; 16];
    rng.fill_bytes(&mut salt);

    // Derive key from password and salt
    let key = derive_encryption_key(password, &salt)?;

    // Generate random nonce
    let mut nonce_bytes = [0u8; 12];
    rng.fill_bytes(&mut nonce_bytes);

    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| WalletError::EncryptionError(format!("Cipher init failed: {}", e)))?;
    let nonce = Nonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(&nonce, data)
        .map_err(|e| WalletError::EncryptionError(format!("Encryption failed: {}", e)))?;

    Ok((
        general_purpose::STANDARD.encode(&ciphertext),
        general_purpose::STANDARD.encode(nonce_bytes),
        general_purpose::STANDARD.encode(salt),
    ))
}

/// Decrypt data with AES-GCM
pub fn decrypt_data(ciphertext: &str, nonce: &str, salt: &str, password: &str) -> Result<Vec<u8>> {
    let ciphertext_bytes = general_purpose::STANDARD
        .decode(ciphertext)
        .map_err(|e| WalletError::DecryptionError(format!("Invalid ciphertext: {}", e)))?;

    let nonce_bytes = general_purpose::STANDARD
        .decode(nonce)
        .map_err(|e| WalletError::DecryptionError(format!("Invalid nonce: {}", e)))?;

    let salt_bytes = general_purpose::STANDARD
        .decode(salt)
        .map_err(|e| WalletError::DecryptionError(format!("Invalid salt: {}", e)))?;
    let (key, legacy_fallback_key) = derive_compatible_keys(password, &salt_bytes)?;
    let nonce_array: [u8; 12] = nonce_bytes
        .as_slice()
        .try_into()
        .map_err(|_| WalletError::DecryptionError("Invalid nonce length".to_string()))?;
    let nonce = Nonce::from(nonce_array);

    let primary_attempt = decrypt_with_key(&ciphertext_bytes, &nonce, &key);

    if primary_attempt.is_ok() {
        return primary_attempt;
    }

    match legacy_fallback_key {
        Some(fallback_key) => decrypt_with_key(&ciphertext_bytes, &nonce, &fallback_key),
        None => primary_attempt,
    }
}

/// Derive encryption key from password
fn derive_encryption_key(password: &str, salt: &[u8]) -> Result<[u8; 32]> {
    let argon2 = Argon2::default();

    let mut key = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| WalletError::CryptoError(format!("Key derivation failed: {}", e)))?;

    Ok(key)
}

fn decrypt_with_key(
    ciphertext_bytes: &[u8],
    nonce: &Nonce<<Aes256Gcm as AeadCore>::NonceSize>,
    key: &[u8; 32],
) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| WalletError::DecryptionError(format!("Cipher init failed: {}", e)))?;

    cipher
        .decrypt(nonce, ciphertext_bytes.as_ref())
        .map_err(|e| WalletError::DecryptionError(format!("Decryption failed: {}", e)))
}

fn derive_compatible_keys(
    password: &str,
    salt_bytes: &[u8],
) -> Result<([u8; 32], Option<[u8; 32]>)> {
    match salt_bytes.len() {
        // New format: a randomly generated 16-byte salt stored alongside the ciphertext.
        16 => Ok((derive_encryption_key(password, salt_bytes)?, None)),
        // Legacy format: a 32-byte derived key was stored instead of a salt. Prefer using the
        // stored key directly, but keep the historical fixed-salt derivation for deterministic
        // wallets created before this change.
        32 => {
            let key: [u8; 32] = salt_bytes
                .try_into()
                .map_err(|_| WalletError::DecryptionError("Invalid legacy key length".into()))?;

            if let Some(legacy_salt) = load_legacy_salt()? {
                let legacy_key = derive_encryption_key(password, &legacy_salt)?;

                if key == legacy_key {
                    Ok((legacy_key, None))
                } else {
                    Ok((key, Some(legacy_key)))
                }
            } else {
                Ok((key, None))
            }
        }
        _ => Ok((derive_encryption_key(password, salt_bytes)?, None)),
    }
}

fn load_legacy_salt() -> Result<Option<Vec<u8>>> {
    #[cfg(test)]
    if let Some(override_bytes) = LEGACY_SALT_OVERRIDE.with(|cell| cell.borrow().clone()) {
        return Ok(Some(override_bytes));
    }

    if let Ok(encoded) = env::var(LEGACY_SALT_ENV) {
        let trimmed = encoded.trim();
        if trimmed.is_empty() {
            return Err(WalletError::CryptoError(
                "Legacy salt override cannot be empty".to_string(),
            ));
        }

        return general_purpose::STANDARD
            .decode(trimmed)
            .map(Some)
            .map_err(|e| {
                WalletError::CryptoError(format!(
                    "Failed to decode legacy salt override as base64: {}",
                    e
                ))
            });
    }

    #[cfg(test)]
    {
        return Ok(Some(LEGACY_SALT_FALLBACK.to_vec()));
    }

    #[cfg(not(test))]
    {
        Ok(None)
    }
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
        // Base58Check addresses don't have a fixed prefix
        assert!(!address.is_empty());
        // Length varies with Base58Check but should be reasonable
        assert!(address.len() > 30 && address.len() < 60);
        assert_eq!(private_key.len(), 32);
        assert_eq!(public_key.len(), 32);
    }

    #[test]
    fn test_encryption_decryption() {
        let data = b"test data";
        let password = "test_password";

        let (ciphertext, nonce, salt) = encrypt_data(data, password).unwrap();
        let decrypted = decrypt_data(&ciphertext, &nonce, &salt, password).unwrap();

        assert_eq!(data, decrypted.as_slice());
    }

    #[test]
    fn test_encryption_uses_unique_salt() {
        let data = b"deterministic";
        let password = "strong_password";

        let (ciphertext_one, nonce_one, salt_one) = encrypt_data(data, password).unwrap();
        let (ciphertext_two, nonce_two, salt_two) = encrypt_data(data, password).unwrap();

        assert_ne!(
            salt_one, salt_two,
            "salts must differ for distinct encryptions"
        );
        assert_ne!(
            ciphertext_one, ciphertext_two,
            "ciphertext should differ with unique salt/nonce"
        );

        let decrypted_one = decrypt_data(&ciphertext_one, &nonce_one, &salt_one, password).unwrap();
        let decrypted_two = decrypt_data(&ciphertext_two, &nonce_two, &salt_two, password).unwrap();

        assert_eq!(data, decrypted_one.as_slice());
        assert_eq!(data, decrypted_two.as_slice());

        let wrong_password = decrypt_data(&ciphertext_one, &nonce_one, &salt_one, "wrong");
        assert!(wrong_password.is_err());
    }

    #[test]
    fn test_legacy_encryption_is_still_decryptable() {
        let data = b"legacy format data";
        let password = "legacy_password";

        // Legacy wallets stored the derived key in the `salt` field and used a fixed salt during derivation.
        let legacy_key = derive_encryption_key(password, LEGACY_SALT_FALLBACK).unwrap();

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let cipher = Aes256Gcm::new_from_slice(&legacy_key).unwrap();
        let nonce = Nonce::from(nonce_bytes);
        let ciphertext = cipher.encrypt(&nonce, data.as_ref()).unwrap();

        let decoded = decrypt_data(
            &general_purpose::STANDARD.encode(ciphertext),
            &general_purpose::STANDARD.encode(nonce_bytes),
            &general_purpose::STANDARD.encode(legacy_key),
            password,
        )
        .unwrap();

        assert_eq!(data, decoded.as_slice());
    }

    #[test]
    fn test_legacy_encryption_with_corrupted_salt_falls_back() {
        let data = b"legacy fallback data";
        let password = "legacy_password";

        let legacy_key = derive_encryption_key(password, LEGACY_SALT_FALLBACK).unwrap();

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let cipher = Aes256Gcm::new_from_slice(&legacy_key).unwrap();
        let nonce = Nonce::from(nonce_bytes);
        let ciphertext = cipher.encrypt(&nonce, data.as_ref()).unwrap();

        // Simulate older wallets that persisted the derived key but later mutated the field.
        let corrupted_salt = [0u8; 32];

        let decoded = decrypt_data(
            &general_purpose::STANDARD.encode(ciphertext),
            &general_purpose::STANDARD.encode(nonce_bytes),
            &general_purpose::STANDARD.encode(corrupted_salt),
            password,
        )
        .unwrap();

        assert_eq!(data, decoded.as_slice());
    }

    #[test]
    fn test_legacy_salt_can_be_overridden() {
        let override_salt = b"custom_legacy_salt__"; // 20 bytes to mirror the legacy format
        let encoded_override = general_purpose::STANDARD.encode(override_salt);
        let _guard = LegacySaltOverrideGuard::new(&encoded_override);

        let password = "legacy_password";
        let derived_key = derive_encryption_key(password, override_salt).unwrap();

        let (key, fallback) = derive_compatible_keys(password, &derived_key).unwrap();

        assert_eq!(key, derived_key);
        assert!(fallback.is_none());
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

    struct LegacySaltOverrideGuard {
        previous_override: Option<Vec<u8>>,
    }

    impl LegacySaltOverrideGuard {
        fn new(value: &str) -> Self {
            let decoded_override = general_purpose::STANDARD
                .decode(value)
                .expect("legacy salt override should decode");
            let previous_override =
                LEGACY_SALT_OVERRIDE.with(|cell| cell.replace(Some(decoded_override)));
            Self { previous_override }
        }
    }

    impl Drop for LegacySaltOverrideGuard {
        fn drop(&mut self) {
            LEGACY_SALT_OVERRIDE.with(|cell| {
                cell.replace(self.previous_override.take());
            });
        }
    }
}
