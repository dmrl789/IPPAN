use blake3::Hasher;
use ed25519_dalek::{SigningKey as Keypair, VerifyingKey as PublicKey, SecretKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub type Hash = [u8; 32];
pub type PublicKeyBytes = [u8; 32];
pub type SignatureBytes = [u8; 64];

// Serde support for byte arrays
mod serde_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        bytes.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        if bytes.len() != 32 {
            return Err(serde::de::Error::custom("Expected 32 bytes"));
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Ok(array)
    }
}

mod serde_signature {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        bytes.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom("Expected 64 bytes"));
        }
        let mut array = [0u8; 64];
        array.copy_from_slice(&bytes);
        Ok(array)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    pub public_key: PublicKeyBytes,
    pub secret_key: Vec<u8>, // Keep secret key as Vec for serialization safety
}

impl KeyPair {
    pub fn generate() -> Self {
        let keypair = Keypair::generate(&mut OsRng);
        Self {
            public_key: keypair.verifying_key().to_bytes(),
            secret_key: keypair.to_bytes().to_vec(),
        }
    }

    pub fn from_secret_key(secret_key_bytes: &[u8]) -> Result<Self, crate::Error> {
        if secret_key_bytes.len() != 32 {
            return Err(crate::Error::Crypto("Invalid secret key length".to_string()));
        }
        
        let secret_key_array: [u8; 32] = secret_key_bytes.try_into()
            .map_err(|_| crate::Error::Crypto("Invalid secret key format".to_string()))?;
        
        let keypair = Keypair::from_bytes(&secret_key_array);
        
        Ok(Self {
            public_key: keypair.verifying_key().to_bytes(),
            secret_key: secret_key_bytes.to_vec(),
        })
    }

    pub fn sign(&self, message: &[u8]) -> Result<SignatureBytes, crate::Error> {
        if self.secret_key.len() != 32 {
            return Err(crate::Error::Crypto("Invalid secret key length".to_string()));
        }
        
        let secret_key_array: [u8; 32] = self.secret_key.as_slice().try_into()
            .map_err(|_| crate::Error::Crypto("Invalid secret key format".to_string()))?;
        
        let keypair = Keypair::from_bytes(&secret_key_array);
        let signature = keypair.sign(message);
        Ok(signature.to_bytes())
    }

    pub fn verify(public_key: &PublicKeyBytes, message: &[u8], signature: &SignatureBytes) -> Result<bool, crate::Error> {
        let public_key = PublicKey::from_bytes(public_key)
            .map_err(|e| crate::Error::Crypto(format!("Invalid public key: {}", e)))?;
        let signature = Signature::try_from(signature.as_slice())
            .map_err(|e| crate::Error::Crypto(format!("Invalid signature: {}", e)))?;
        
        Ok(public_key.verify(message, &signature).is_ok())
    }

    pub fn batch_verify(
        public_keys: &[PublicKeyBytes],
        messages: &[&[u8]],
        signatures: &[SignatureBytes],
    ) -> Result<Vec<bool>, crate::Error> {
        if public_keys.len() != messages.len() || messages.len() != signatures.len() {
            return Err(crate::Error::Crypto("Mismatched array lengths".to_string()));
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
}

pub fn hash(data: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

pub fn blake3_hash(data: &[u8]) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Generate HashTimer as specified in the PRD:
/// HashTimer = H( prefix(IPPAN_time_us) ∥ entropy ∥ tx_id )
pub fn generate_hash_timer(ippan_time_us: u64, entropy: &[u8], tx_id: &Hash) -> Hash {
    let mut hasher = Hasher::new();
    
    // prefix(IPPAN_time_us) - use first 8 bytes
    hasher.update(&ippan_time_us.to_le_bytes());
    
    // entropy
    hasher.update(entropy);
    
    // tx_id
    hasher.update(tx_id);
    
    hasher.finalize().into()
}

/// Derive address from public key using base58i encoding
pub fn derive_address(public_key: &PublicKeyBytes) -> String {
    let mut hasher = Sha256::new();
    hasher.update(public_key);
    let hash = hasher.finalize();
    
    // Add 'i' prefix for IPPAN addresses
    let mut address_bytes = vec![b'i'];
    address_bytes.extend_from_slice(&hash[..20]); // Use first 20 bytes
    
    bs58::encode(&address_bytes).into_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = KeyPair::generate();
        assert_eq!(keypair.public_key.len(), 32);
        assert_eq!(keypair.secret_key.len(), 32);
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
    fn test_hash_timer_generation() {
        let ippan_time = 1234567890;
        let entropy = b"random_entropy";
        let tx_id = [1u8; 32];
        
        let hash_timer = generate_hash_timer(ippan_time, entropy, &tx_id);
        assert_eq!(hash_timer.len(), 32);
        
        // Same inputs should produce same hash timer
        let hash_timer2 = generate_hash_timer(ippan_time, entropy, &tx_id);
        assert_eq!(hash_timer, hash_timer2);
    }

    #[test]
    fn test_address_derivation() {
        let keypair = KeyPair::generate();
        let address = derive_address(&keypair.public_key);
        
        assert!(address.starts_with('i'));
        assert!(address.len() > 1);
    }

    #[test]
    fn test_batch_verify() {
        let keypairs: Vec<KeyPair> = (0..5).map(|_| KeyPair::generate()).collect();
        let messages: Vec<&[u8]> = (0..5).map(|i| format!("message{}", i).as_bytes()).collect();
        
        let signatures: Vec<SignatureBytes> = keypairs
            .iter()
            .zip(messages.iter())
            .map(|(kp, msg)| kp.sign(msg).unwrap())
            .collect();
        
        let public_keys: Vec<PublicKeyBytes> = keypairs.iter().map(|kp| kp.public_key).collect();
        
        let results = KeyPair::batch_verify(&public_keys, &messages, &signatures).unwrap();
        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|&valid| valid));
    }
}
