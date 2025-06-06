use rand::rngs::OsRng;
use rand::RngCore;
use ed25519_dalek::{SigningKey, VerifyingKey, SECRET_KEY_LENGTH};
use bip39::Mnemonic;
use std::fs::{self, File};
use std::io::{Read, Write};

pub struct Wallet {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
    pub address: String,
}

impl Wallet {
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let mut sk_bytes = [0u8; SECRET_KEY_LENGTH];
        csprng.fill_bytes(&mut sk_bytes);
        let signing_key = SigningKey::from_bytes(&sk_bytes);
        let verifying_key = VerifyingKey::from(&signing_key);

        // Generate address as base58 of verifying_key (or your own logic)
        let address = bs58::encode(verifying_key.as_bytes()).into_string();

        Wallet {
            signing_key,
            verifying_key,
            address,
        }
    }

    pub fn save_to_file(&self, path: &str) {
        let sk_bytes = self.signing_key.to_bytes();
        fs::write(path, &sk_bytes).unwrap();
    }

    pub fn load_from_file(path: &str) -> Self {
        let mut sk_bytes = [0u8; SECRET_KEY_LENGTH];
        let mut file = File::open(path).unwrap();
        file.read_exact(&mut sk_bytes).unwrap();
        let signing_key = SigningKey::from_bytes(&sk_bytes);
        let verifying_key = VerifyingKey::from(&signing_key);
        let address = bs58::encode(verifying_key.as_bytes()).into_string();
        Wallet {
            signing_key,
            verifying_key,
            address,
        }
    }

    pub fn print_mnemonic(&self) {
        let sk_bytes = self.signing_key.to_bytes();
        let mnemonic = Mnemonic::from_entropy(&sk_bytes).unwrap();
        println!("🔐 Private Key Mnemonic (BIP39):\n{}", mnemonic);
    }

    pub fn print_private_hex(&self) {
        let sk_bytes = self.signing_key.to_bytes();
        println!("🔑 Private Key (hex): {:x?}", sk_bytes);
    }
}
