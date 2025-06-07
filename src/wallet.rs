use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use bip39::Mnemonic;
use bs58;
use std::fs::File;
use std::io::{Read, Write};

#[derive(Debug)]
pub struct Wallet {
    pub signing_key: SigningKey,
    //pub verifying_key: VerifyingKey,
    pub address: String,
}

impl Wallet {
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);
        let address = bs58::encode(verifying_key.as_bytes()).into_string();

        Wallet {
            signing_key,
            verifying_key,
            address,
        }
    }

    pub fn save_to_file(&self, path: &str) {
        let sk_bytes = self.signing_key.to_bytes();
        let mut file = File::create(path).expect("Unable to create wallet file");
        file.write_all(&sk_bytes).expect("Unable to write wallet file");
    }

    pub fn load_from_file(path: &str) -> Self {
        let mut file = File::open(path).expect("Unable to open wallet file");
        let mut sk_bytes = [0u8; 32];
        file.read_exact(&mut sk_bytes).expect("Unable to read wallet file");
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
        let mnemonic = Mnemonic::from_entropy(&sk_bytes[0..16]).unwrap();
        println!("🔐 Private Key Mnemonic (BIP39):\n{}", mnemonic.to_string());
    }

    pub fn print_private_hex(&self) {
        let sk_bytes = self.signing_key.to_bytes();
        println!("🔑 Private Key (hex): {}", hex::encode(sk_bytes));
    }
}
