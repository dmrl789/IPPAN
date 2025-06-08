use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use bs58;

#[derive(Debug, Clone)]
pub struct Wallet {
    pub signing_key: SigningKey,
}

impl Wallet {
    pub fn new() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        Self { signing_key }
    }

    pub fn address(&self) -> String {
        let verifying_key: VerifyingKey = self.signing_key.verifying_key();
        let pk_bytes = verifying_key.to_bytes();
        bs58::encode(pk_bytes).into_string()
    }
}
