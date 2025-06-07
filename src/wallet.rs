use bip39::{Language, Mnemonic};
use ed25519_dalek::{SigningKey, VerifyingKey, SECRET_KEY_LENGTH};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};

#[derive(Debug)]
pub struct Wallet {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
    pub address: String,
}

impl Wallet {
    /// Generate a new wallet (with random private key)
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let mut sk_bytes = [0u8; SECRET_KEY_LENGTH];
        csprng.fill_bytes(&mut sk_bytes);
        let signing_key = SigningKey::from_bytes(&sk_bytes);
        let verifying_key = VerifyingKey::from(&signing_key);

        // Simple address: use the verifying key (public key) as base58 string
        let address = bs58::encode(verifying_key.as_bytes()).into_string();

        Wallet {
            signing_key,
            verifying_key,
            address,
        }
    }

    /// Save wallet to file (binary, just private key and address)
    pub fn save_to_file(&self, path: &str) {
        let sk_bytes = self.signing_key.to_bytes();
        let address = self.address.as_bytes();
        let mut file = File::create(path).expect("Unable to create wallet file");
        // Write: sk_bytes (32 bytes) + address (as utf8, length up to 100)
        file.write_all(&sk_bytes).expect("Failed to write private key");
        file.write_all(address).expect("Failed to write address");
    }

    /// Load wallet from file (expects private key + address)
    pub fn load_from_file(path: &str) -> Self {
        let mut file = File::open(path).expect("Unable to open wallet file");
        let mut sk_bytes = [0u8; SECRET_KEY_LENGTH];
        file.read_exact(&mut sk_bytes).expect("Failed to read private key");
        let mut address_bytes = Vec::new();
        file.read_to_end(&mut address_bytes).expect("Failed to read address");
        let signing_key = SigningKey::from_bytes(&sk_bytes);
        let verifying_key = VerifyingKey::from(&signing_key);
        let address = String::from_utf8(address_bytes).unwrap_or_default();
        Wallet {
            signing_key,
            verifying_key,
            address,
        }
    }

    /// Print private key as BIP39 mnemonic
    pub fn print_mnemonic(&self) {
        let sk_bytes = self.signing_key.to_bytes();
        // BIP39 expects 16/20/24/32 bytes; we'll use first 16 bytes for 12 words (standard)
        let mnemonic = Mnemonic::from_entropy(&sk_bytes[0..16]).unwrap();
        println!("🔐 Private Key Mnemonic (BIP39):\n{}", mnemonic.to_string());
    }

    /// Print private key as hex
    pub fn print_private_hex(&self) {
        let sk_bytes = self.signing_key.to_bytes();
        println!("🔑 Private Key (hex): {}", hex::encode(sk_bytes));
    }
}
