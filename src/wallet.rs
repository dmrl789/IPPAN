use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use bs58;
use bip39::Mnemonic;
use std::fs::File;
use std::io::{Read, Write};

#[derive(Debug, Clone)]
pub struct Wallet {
    pub signing_key: SigningKey,
    pub address: String,
}

impl Wallet {
    pub fn generate() -> Self {
        let mut csprng = OsRng {};
        let signing_key = SigningKey::generate(&mut csprng);
        let public_key = signing_key.verifying_key();
        let address = bs58::encode(public_key.as_bytes()).into_string();

        Wallet {
            signing_key,
            address,
        }
    }

    pub fn save_to_file(&self, path: &str) {
        let sk_bytes = self.signing_key.to_bytes();
        let address = &self.address;
        let mut file = File::create(path).expect("Unable to create file");
        file.write_all(&sk_bytes).expect("Unable to write signing key");
        file.write_all(address.as_bytes()).expect("Unable to write address");
    }

    pub fn load_from_file(path: &str) -> Self {
        let mut file = File::open(path).expect("Unable to open file");
        let mut sk_bytes = [0u8; 32];
        file.read_exact(&mut sk_bytes).expect("Unable to read signing key");
        let mut address_bytes = Vec::new();
        file.read_to_end(&mut address_bytes).expect("Unable to read address");
        let signing_key = SigningKey::from_bytes(&sk_bytes);
        let address = String::from_utf8(address_bytes).unwrap_or_default();
        Wallet {
            signing_key,
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
