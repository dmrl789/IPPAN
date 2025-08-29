use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use blake3::Hasher as Blake3;
use blake2::{Blake2b, Digest};
use rand::Rng;
use bs58;
use serde::{Deserialize, Serialize};
use crate::Result;
use tracing;

/// Hash type (32 bytes)
pub type Hash = [u8; 32];

/// Public key bytes (32 bytes)
pub type PublicKeyBytes = [u8; 32];

/// Signature bytes (64 bytes)
pub type SignatureBytes = [u8; 64];

/// Ed25519 KeyPair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    #[serde(with = "serde_bytes")]
    pub secret_key: [u8; 32],
    #[serde(with = "serde_bytes")]
    pub public_key: PublicKeyBytes,
}

impl KeyPair {
    /// Generate a new keypair
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
        let verifying_key = signing_key.verifying_key();
        
        Self {
            secret_key: signing_key.to_bytes(),
            public_key: verifying_key.to_bytes(),
        }
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> Result<SignatureBytes> {
        let signing_key = SigningKey::from_bytes(&self.secret_key);
        let signature = signing_key.sign(message);
        Ok(signature.to_bytes())
    }

    /// Verify a signature
    pub fn verify(public_key: &PublicKeyBytes, message: &[u8], signature: &SignatureBytes) -> Result<bool> {
        let verifying_key = VerifyingKey::from_bytes(public_key)?;
        let sig = ed25519_dalek::Signature::from_bytes(signature);
        
        Ok(verifying_key.verify(message, &sig).is_ok())
    }

    /// Batch verify signatures
    pub fn batch_verify(
        public_keys: &[PublicKeyBytes],
        messages: &[&[u8]],
        signatures: &[SignatureBytes],
    ) -> Result<Vec<bool>> {
        if public_keys.len() != messages.len() || messages.len() != signatures.len() {
            return Err(crate::Error::Crypto("Batch verify: mismatched array lengths".to_string()));
        }

        let mut results = Vec::with_capacity(public_keys.len());
        
        for (i, ((pk, msg), sig)) in public_keys.iter().zip(messages.iter()).zip(signatures.iter()).enumerate() {
            match Self::verify(pk, msg, sig) {
                Ok(valid) => results.push(valid),
                Err(e) => {
                    tracing::warn!("Batch verify failed at index {}: {}", i, e);
                    results.push(false);
                }
            }
        }
        
        Ok(results)
    }

    /// Get the public key
    pub fn public_key(&self) -> PublicKeyBytes {
        self.public_key
    }

    /// Get the secret key bytes
    pub fn secret_key_bytes(&self) -> [u8; 32] {
        self.secret_key
    }
}

/// Blake2b hash function
pub fn hash(data: &[u8]) -> Hash {
    let mut hasher = Blake2b::new();
    hasher.update(data);
    let result: [u8; 64] = hasher.finalize().into();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result[..32]);
    hash
}

/// Blake3 hash function
pub fn blake3_hash(data: &[u8]) -> Hash {
    let mut hasher = Blake3::new();
    hasher.update(data);
    let result: [u8; 32] = hasher.finalize().into();
    result
}

/// Generate hashtimer for a transaction
pub fn generate_hash_timer(ippan_time_us: u64, entropy: &[u8], tx_id: &Hash) -> Hash {
    let mut input = Vec::new();
    input.extend_from_slice(&ippan_time_us.to_le_bytes());
    input.extend_from_slice(entropy);
    input.extend_from_slice(tx_id);
    blake3_hash(&input)
}

/// Derive base58i address from public key
pub fn derive_address(public_key: &PublicKeyBytes) -> String {
    let hash = blake3_hash(public_key);
    let encoded = bs58::encode(&hash).into_string();
    format!("i{}", encoded)
}

/// Compute HashTimer for a transaction
pub fn hashtimer(tx_id: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Blake3::new();
    hasher.update(b"time_prefix");
    hasher.update(&rand::random::<[u8; 32]>()); // entropy
    hasher.update(tx_id);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = KeyPair::generate();
        assert_ne!(keypair.secret_key, [0u8; 32]);
        assert_ne!(keypair.public_key, [0u8; 32]);
    }

    #[test]
    fn test_sign_and_verify() {
        let keypair = KeyPair::generate();
        let message = b"Hello, IPPAN!";
        
        let signature = keypair.sign(message).unwrap();
        let is_valid = KeyPair::verify(&keypair.public_key, message, &signature).unwrap();
        
        assert!(is_valid);
    }

    #[test]
    fn test_batch_verify() {
        let keypairs: Vec<KeyPair> = (0..10).map(|_| KeyPair::generate()).collect();
        let messages: Vec<Vec<u8>> = (0..10).map(|i| format!("message {}", i).into_bytes()).collect();
        let signatures: Vec<SignatureBytes> = keypairs.iter()
            .zip(messages.iter())
            .map(|(kp, msg)| kp.sign(msg).unwrap())
            .collect();
        
        let public_keys: Vec<PublicKeyBytes> = keypairs.iter()
            .map(|kp| kp.public_key)
            .collect();
        
        let message_refs: Vec<&[u8]> = messages.iter().map(|m| m.as_slice()).collect();
        let results = KeyPair::batch_verify(&public_keys, &message_refs, &signatures).unwrap();
        
        assert_eq!(results.len(), 10);
        assert!(results.iter().all(|&valid| valid));
    }

    #[test]
    fn test_hash_functions() {
        let data = b"test data";
        let hash1 = hash(data);
        let hash2 = blake3_hash(data);
        
        assert_ne!(hash1, [0u8; 32]);
        assert_ne!(hash2, [0u8; 32]);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hashtimer_generation() {
        let ippan_time = 1234567890u64;
        let entropy = b"entropy";
        let tx_id = [1u8; 32];
        
        let hashtimer = generate_hash_timer(ippan_time, entropy, &tx_id);
        assert_ne!(hashtimer, [0u8; 32]);
    }

    #[test]
    fn test_address_derivation() {
        let keypair = KeyPair::generate();
        let address = derive_address(&keypair.public_key);
        
        assert!(address.starts_with('i'));
        assert!(address.len() > 1);
    }
}
