use crate::crypto::generate_new_address;
use crate::errors::{Result, WalletError};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{serde::ts_seconds, DateTime, Utc};
use ed25519_dalek::SigningKey;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Current on-disk key file schema version.
const KEYFILE_VERSION: u8 = 1;
const KDF_LABEL: &str = "argon2id-v1";
const PLAINTEXT_WARNING: &str = "Key file stored without password protection";

/// Serialized key file written to disk.
#[derive(Debug, Serialize, Deserialize)]
pub struct KeyFile {
    pub version: u8,
    pub address: String,
    pub public_key_hex: String,
    #[serde(default)]
    pub metadata: KeyMetadata,
    #[serde(flatten)]
    pub secret: KeySecret,
}

/// Metadata describing when/where the key was created.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeyMetadata {
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

impl KeyMetadata {
    pub fn new(network_profile: Option<String>, warning: Option<String>) -> Self {
        Self {
            created_at: Utc::now(),
            network_profile,
            notes: None,
            warning,
        }
    }
}

impl Default for KeyMetadata {
    fn default() -> Self {
        KeyMetadata::new(None, None)
    }
}

/// Secret material stored in the key file.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "protection", rename_all = "snake_case")]
pub enum KeySecret {
    Plain {
        private_key_hex: String,
    },
    PasswordProtected {
        ciphertext: String,
        nonce: String,
        salt: String,
        kdf: String,
    },
}

/// Unlocked key material ready for signing.
#[derive(Debug, Clone)]
pub struct UnlockedKey {
    pub private_key: [u8; 32],
    pub public_key: [u8; 32],
    pub address: String,
    pub metadata: KeyMetadata,
}

impl KeyFile {
    /// Create a new key file from freshly generated key material.
    pub fn generate(
        password: Option<&str>,
        network_profile: Option<String>,
        allow_plaintext: bool,
    ) -> Result<(Self, UnlockedKey)> {
        let (address, private_key, public_key) = generate_new_address()
            .map_err(|e| WalletError::AddressGenerationFailed(e.to_string()))?;
        let secret = build_secret(&private_key, password, allow_plaintext)?;
        let metadata = KeyMetadata::new(
            network_profile,
            if matches!(secret, KeySecret::Plain { .. }) {
                Some(PLAINTEXT_WARNING.to_string())
            } else {
                None
            },
        );

        let keyfile = Self {
            version: KEYFILE_VERSION,
            address: address.clone(),
            public_key_hex: hex::encode(public_key),
            metadata: metadata.clone(),
            secret,
        };

        Ok((
            keyfile,
            UnlockedKey {
                private_key,
                public_key,
                address,
                metadata,
            },
        ))
    }

    /// Persist the key file to disk atomically.
    pub fn save(&self, path: &Path, force: bool) -> Result<()> {
        if path.exists() && !force {
            return Err(WalletError::StorageError(format!(
                "key file {} already exists (use --force to overwrite)",
                path.display()
            )));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp_path = tmp_path(path);
        let data = serde_json::to_vec_pretty(self)?;
        fs::write(&tmp_path, data)?;
        fs::rename(tmp_path, path)?;
        Ok(())
    }

    /// Load a key file from disk.
    pub fn load(path: &Path) -> Result<Self> {
        let data = fs::read(path)?;
        let keyfile: KeyFile = serde_json::from_slice(&data)?;
        if keyfile.version != KEYFILE_VERSION {
            return Err(WalletError::StorageError(format!(
                "unsupported key file version {} (expected {})",
                keyfile.version, KEYFILE_VERSION
            )));
        }
        Ok(keyfile)
    }

    /// Unlock the private key using an optional password.
    pub fn unlock(&self, password: Option<&str>) -> Result<UnlockedKey> {
        let private_key = match &self.secret {
            KeySecret::Plain { private_key_hex } => {
                let bytes = hex::decode(private_key_hex)
                    .map_err(|err| WalletError::InvalidPrivateKey(err.to_string()))?;
                if bytes.len() != 32 {
                    return Err(WalletError::InvalidPrivateKey(format!(
                        "expected 32 bytes, got {}",
                        bytes.len()
                    )));
                }
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                key
            }
            KeySecret::PasswordProtected {
                ciphertext,
                nonce,
                salt,
                ..
            } => {
                let pwd = password.ok_or(WalletError::InvalidPassword)?;
                decrypt_private_key(ciphertext, nonce, salt, pwd)?
            }
        };

        let signing_key = SigningKey::try_from(private_key.as_slice())
            .map_err(|e| WalletError::InvalidPrivateKey(e.to_string()))?;
        let public_key = signing_key.verifying_key().to_bytes();
        if self.public_key_hex != hex::encode(public_key) {
            return Err(WalletError::InvalidPrivateKey(
                "public key in file does not match decrypted private key".into(),
            ));
        }

        Ok(UnlockedKey {
            private_key,
            public_key,
            address: self.address.clone(),
            metadata: self.metadata.clone(),
        })
    }
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(".tmp");
    PathBuf::from(tmp)
}

fn build_secret(
    private_key: &[u8; 32],
    password: Option<&str>,
    allow_plaintext: bool,
) -> Result<KeySecret> {
    if let Some(pwd) = password {
        encrypt_private_key(private_key, pwd)
    } else if allow_plaintext {
        Ok(KeySecret::Plain {
            private_key_hex: hex::encode(private_key),
        })
    } else {
        Err(WalletError::InvalidPassword)
    }
}

fn encrypt_private_key(private_key: &[u8; 32], password: &str) -> Result<KeySecret> {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);

    let key = derive_encryption_key(password, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|err| WalletError::EncryptionError(format!("cipher init failed: {err}")))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, private_key.as_slice())
        .map_err(|err| WalletError::EncryptionError(format!("encryption failed: {err}")))?;

    Ok(KeySecret::PasswordProtected {
        ciphertext: BASE64.encode(ciphertext),
        nonce: BASE64.encode(nonce_bytes),
        salt: BASE64.encode(salt),
        kdf: KDF_LABEL.to_string(),
    })
}

fn decrypt_private_key(
    ciphertext: &str,
    nonce: &str,
    salt: &str,
    password: &str,
) -> Result<[u8; 32]> {
    let ciphertext_bytes = BASE64
        .decode(ciphertext)
        .map_err(|err| WalletError::DecryptionError(format!("invalid ciphertext: {err}")))?;
    let nonce_bytes = BASE64
        .decode(nonce)
        .map_err(|err| WalletError::DecryptionError(format!("invalid nonce: {err}")))?;
    let salt_bytes = BASE64
        .decode(salt)
        .map_err(|err| WalletError::DecryptionError(format!("invalid salt: {err}")))?;

    let key = derive_encryption_key(password, &salt_bytes)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|err| WalletError::DecryptionError(format!("cipher init failed: {err}")))?;
    let nonce_array: [u8; 12] = nonce_bytes
        .as_slice()
        .try_into()
        .map_err(|_| WalletError::DecryptionError("nonce must be 12 bytes".into()))?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce_array), ciphertext_bytes.as_ref())
        .map_err(|err| WalletError::DecryptionError(format!("decryption failed: {err}")))?;
    if plaintext.len() != 32 {
        return Err(WalletError::InvalidPrivateKey(format!(
            "expected 32 byte key, got {} bytes",
            plaintext.len()
        )));
    }
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&plaintext);
    Ok(key_bytes)
}

fn derive_encryption_key(password: &str, salt: &[u8]) -> Result<[u8; 32]> {
    let mut key = [0u8; 32];
    let argon2 = argon2::Argon2::default();
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|err| WalletError::CryptoError(format!("key derivation failed: {err}")))?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn round_trip_encrypted_keyfile() {
        let (keyfile, unlocked) =
            KeyFile::generate(Some("hunter2"), Some("devnet".into()), false).unwrap();
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.key");
        keyfile.save(&path, false).unwrap();

        let loaded = KeyFile::load(&path).unwrap();
        let unlocked_again = loaded.unlock(Some("hunter2")).unwrap();
        assert_eq!(unlocked.address, unlocked_again.address);
        assert_eq!(
            hex::encode(unlocked.private_key),
            hex::encode(unlocked_again.private_key)
        );
    }

    #[test]
    fn plaintext_requires_flag() {
        let result = KeyFile::generate(None, None, false);
        assert!(result.is_err());

        let (keyfile, unlocked) = KeyFile::generate(None, None, true).unwrap();
        assert!(matches!(keyfile.secret, KeySecret::Plain { .. }));
        assert!(keyfile.metadata.warning.is_some());
        let unlocked_again = keyfile.unlock(None).unwrap();
        assert_eq!(
            hex::encode(unlocked.private_key),
            hex::encode(unlocked_again.private_key)
        );
    }
}
