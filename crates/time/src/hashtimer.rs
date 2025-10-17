//! HashTimer — cryptographically verifiable timestamp primitive.
//!
//! A `HashTimer` pairs the current deterministic IPPAN time with
//! freshly generated entropy and signs the resulting digest. The
//! signature allows other nodes to verify the origin and integrity
//! of the timestamp, providing a lightweight ordering primitive that
//! can be attached to blocks, transactions, or gossip payloads.

use std::convert::TryInto;

use blake3::Hasher;
use ed25519_dalek::{
    Signature, Signer, SigningKey, Verifier, VerifyingKey, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH,
};
use hex::ToHex;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::ippan_time::now_us;

/// Number of bytes used for entropy embedded in each [`HashTimer`].
pub const HASHTIMER_ENTROPY_BYTES: usize = 32;

/// HashTimer payload that accompanies deterministic IPPAN time updates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashTimer {
    /// IPPAN time in microseconds when this structure was created.
    pub timestamp_us: i64,
    /// Random entropy mixed into the digest for uniqueness.
    pub entropy: [u8; HASHTIMER_ENTROPY_BYTES],
    /// Ed25519 signature bytes authenticating the timestamp and entropy.
    pub signature: Vec<u8>,
    /// Public key corresponding to the signer of this HashTimer.
    pub public_key: Vec<u8>,
}

impl HashTimer {
    /// Compute the canonical hash digest of this HashTimer.
    pub fn digest(&self) -> [u8; 32] {
        digest_from_parts(self.timestamp_us, &self.entropy)
    }

    /// Render the digest as a lowercase hexadecimal identifier.
    pub fn id_hex(&self) -> String {
        self.digest().encode_hex::<String>()
    }

    /// Verify the signature embedded in this HashTimer.
    pub fn verify(&self) -> bool {
        verify_hashtimer(self)
    }
}

/// Generate fresh entropy suitable for a new [`HashTimer`].
pub fn generate_entropy() -> [u8; HASHTIMER_ENTROPY_BYTES] {
    let mut entropy = [0u8; HASHTIMER_ENTROPY_BYTES];
    OsRng.fill_bytes(&mut entropy);
    entropy
}

/// Create a new [`HashTimer`] signed by the provided keypair.
pub fn sign_hashtimer(signing_key: &SigningKey) -> HashTimer {
    let timestamp_us = now_us();
    let entropy = generate_entropy();
    let digest = digest_from_parts(timestamp_us, &entropy);

    let signature = signing_key.sign(&digest);
    let verifying_key = signing_key.verifying_key();

    HashTimer {
        timestamp_us,
        entropy,
        signature: signature.to_bytes().to_vec(),
        public_key: verifying_key.to_bytes().to_vec(),
    }
}

/// Verify the authenticity of a [`HashTimer`].
pub fn verify_hashtimer(timer: &HashTimer) -> bool {
    let Ok(signature_bytes) = timer.signature.as_slice().try_into() else {
        return false;
    };
    let Ok(public_key_bytes) = timer.public_key.as_slice().try_into() else {
        return false;
    };

    let Ok(signature) = Signature::from_bytes(&signature_bytes) else {
        return false;
    };
    let Ok(public_key) = VerifyingKey::from_bytes(&public_key_bytes) else {
        return false;
    };

    let digest = digest_from_parts(timer.timestamp_us, &timer.entropy);
    public_key.verify(&digest, &signature).is_ok()
}

fn digest_from_parts(timestamp_us: i64, entropy: &[u8; HASHTIMER_ENTROPY_BYTES]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(&timestamp_us.to_be_bytes());
    hasher.update(entropy);
    let mut output = [0u8; 32];
    output.copy_from_slice(hasher.finalize().as_bytes());
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entropy_generation_produces_unique_values() {
        let first = generate_entropy();
        let second = generate_entropy();
        assert_ne!(first, second);
    }

    #[test]
    fn signing_and_verifying_round_trip() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let timer = sign_hashtimer(&signing_key);
        assert!(verify_hashtimer(&timer));
        assert!(timer.verify());
    }

    #[test]
    fn tampering_invalidates_signature() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let mut timer = sign_hashtimer(&signing_key);
        timer.timestamp_us += 1;
        assert!(!verify_hashtimer(&timer));
    }
}
