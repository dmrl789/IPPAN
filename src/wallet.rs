use bip39::{Mnemonic, Seed};
use ed25519_dalek::{SigningKey, VerifyingKey, Signer, SECRET_KEY_LENGTH};
use rand::rngs::OsRng;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use std::io::{self, Write};
use bs58;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WalletData {
    pub mnemonic: String, // BIP39 backup
    pub secret: Vec<u8>,  // 32 bytes
}

#[derive(Debug, Clone)]
pub struct Wallet {
    pub signing_key: SigningKey,
    pub mnemonic: String,
}

impl Wallet {
    pub fn new() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let sk_bytes = signing_key.to_bytes();
        let mnemonic = Mnemonic::from_entropy(&sk_bytes).unwrap().to_string();
        Self { signing_key, mnemonic }
    }

    pub fn from_mnemonic(mnemonic: &str) -> Self {
        let mnemonic = Mnemonic::parse(mnemonic).unwrap();
        let seed = Seed::new(&mnemonic, "");
        let sk_bytes = &seed.as_bytes()[0..SECRET_KEY_LENGTH];
        let signing_key = SigningKey::from_bytes(sk_bytes).unwrap();
        Self { signing_key, mnemonic: mnemonic.to_string() }
    }

    pub fn address(&self) -> String {
        let verifying_key = self.signing_key.verifying_key();
        let pk_bytes = verifying_key.to_bytes();
        bs58::encode(pk_bytes).into_string()
    }

    pub fn sign(&self, msg: &[u8]) -> Vec<u8> {
        self.signing_key.sign(msg).to_bytes().to_vec()
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    // Save to file (encrypted with passphrase, basic)
    pub fn save_to_file(&self, file: &str, passphrase: &str) {
        let data = WalletData {
            mnemonic: self.mnemonic.clone(),
            secret: self.signing_key.to_bytes().to_vec(),
        };
        let json = serde_json::to_string(&data).unwrap();
        let xored: Vec<u8> = json.bytes()
            .zip(passphrase.bytes().cycle())
            .map(|(b, p)| b ^ p).collect();
        fs::write(file, xored).unwrap();
    }

    // Load from file (decrypt with passphrase)
    pub fn load_from_file(file: &str, passphrase: &str) -> Self {
        let bytes = fs::read(file).unwrap();
        let decoded: Vec<u8> = bytes.iter()
            .zip(passphrase.bytes().cycle())
            .map(|(b, p)| b ^ p).collect();
        let json = String::from_utf8(decoded).unwrap();
        let data: WalletData = serde_json::from_str(&json).unwrap();
        Self::from_mnemonic(&data.mnemonic)
    }
}
