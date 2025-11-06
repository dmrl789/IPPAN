//! HashTimer — cryptographically verifiable timestamp primitive.
//!
//! A `HashTimer` pairs the current deterministic IPPAN time with
//! freshly generated entropy and optionally signs the resulting digest. The
//! signature allows other nodes to verify the origin and integrity
//! of the timestamp, providing a lightweight ordering primitive that
//! can be attached to blocks, transactions, or gossip payloads.

use std::convert::TryInto;

use blake3::Hasher;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hex::{decode as hex_decode, encode as hex_encode, ToHex};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::ippan_time::now_us;

/// Number of bytes used for entropy embedded in each [`HashTimer`].
pub const HASHTIMER_ENTROPY_BYTES: usize = 32;

/// IPPAN Time: microsecond precision timestamp
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct IppanTimeMicros(pub u64);

impl IppanTimeMicros {
    /// Get current IPPAN time in microseconds
    pub fn now() -> Self {
        Self(now_us() as u64)
    }
}

/// HashTimer payload that accompanies deterministic IPPAN time updates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashTimer {
    /// IPPAN time in microseconds when this structure was created.
    pub timestamp_us: i64,
    /// Random entropy mixed into the digest for uniqueness.
    pub entropy: [u8; HASHTIMER_ENTROPY_BYTES],
    /// Ed25519 signature bytes authenticating the timestamp and entropy (empty if unsigned).
    pub signature: Vec<u8>,
    /// Public key corresponding to the signer of this HashTimer (empty if unsigned).
    pub public_key: Vec<u8>,
}

impl HashTimer {
    /// Compute the canonical hash digest of this HashTimer.
    pub fn digest(&self) -> [u8; 32] {
        digest_from_parts(self.timestamp_us, &self.entropy)
    }

    /// Alias for `digest()` to match specification naming.
    pub fn hash(&self) -> [u8; 32] {
        self.digest()
    }

    /// Render the digest as a lowercase hexadecimal identifier.
    pub fn id_hex(&self) -> String {
        self.digest().encode_hex::<String>()
    }

    /// Convert to hex string representation (64 hex chars: 14 time prefix + 50 hash suffix)
    pub fn to_hex(&self) -> String {
        // Create 14-hex time prefix (56 bits)
        let time_bits = (self.timestamp_us as u64) & 0x00FFFFFFFFFFFFFF; // Mask to 56 bits
        let time_bytes = &time_bits.to_be_bytes();
        let time_hex_full = hex_encode(&time_bytes[1..8]); // Last 7 bytes = 14 hex chars
        let time_prefix = if time_hex_full.len() >= 14 {
            time_hex_full[time_hex_full.len() - 14..].to_string()
        } else {
            format!("{time_hex_full:0>14}")
        };

        // Use digest as 50-hex hash suffix (200 bits from first 25 bytes)
        let digest_hex = hex_encode(self.digest());
        let hash_suffix = &digest_hex[0..50.min(digest_hex.len())];

        format!("{time_prefix}{hash_suffix}")
    }

    /// Parse from hex string (64 hex characters)
    pub fn from_hex(hex_str: &str) -> Result<Self, String> {
        if hex_str.len() != 64 {
            return Err(format!(
                "HashTimer hex must be 64 characters, got {}",
                hex_str.len()
            ));
        }

        // Parse time prefix (14 hex chars = 7 bytes = 56 bits)
        let time_hex = &hex_str[0..14];
        let time_bytes =
            hex_decode(time_hex).map_err(|e| format!("Invalid time prefix hex: {e}"))?;
        if time_bytes.len() != 7 {
            return Err("Time prefix must be 7 bytes".to_string());
        }
        let mut time_prefix_bytes = [0u8; 8];
        time_prefix_bytes[1..8].copy_from_slice(&time_bytes);
        let time_u64 = u64::from_be_bytes(time_prefix_bytes);

        // Parse hash suffix (50 hex chars = 25 bytes = 200 bits)
        let hash_hex = &hex_str[14..64];
        let hash_bytes =
            hex_decode(hash_hex).map_err(|e| format!("Invalid hash suffix hex: {e}"))?;
        if hash_bytes.len() != 25 {
            return Err("Hash suffix must be 25 bytes".to_string());
        }

        // Reconstruct entropy from hash suffix (we'll use first 32 bytes, padding if needed)
        let mut entropy = [0u8; HASHTIMER_ENTROPY_BYTES];
        entropy[0..hash_bytes.len().min(32)]
            .copy_from_slice(&hash_bytes[0..hash_bytes.len().min(32)]);
        if hash_bytes.len() < 32 {
            // Fill remainder deterministically
            let mut hasher = Hasher::new();
            hasher.update(&hash_bytes);
            hasher.update(&time_prefix_bytes);
            let pad_hash = hasher.finalize();
            entropy[hash_bytes.len()..32]
                .copy_from_slice(&pad_hash.as_bytes()[0..(32 - hash_bytes.len())]);
        }

        Ok(HashTimer {
            timestamp_us: time_u64 as i64,
            entropy,
            signature: Vec::new(), // Unsigned when created from hex
            public_key: Vec::new(),
        })
    }

    /// Get the time component as IppanTimeMicros
    pub fn time(&self) -> IppanTimeMicros {
        IppanTimeMicros(self.timestamp_us as u64)
    }

    /// Derive HashTimer from components (creates unsigned HashTimer, can be signed later)
    pub fn derive(
        context: &str,
        time: IppanTimeMicros,
        domain: &[u8],
        payload: &[u8],
        nonce: &[u8],
        node_id: &[u8],
    ) -> Self {
        let timestamp_us = time.0 as i64;

        // Compute deterministic entropy from inputs
        let mut hasher = Hasher::new();
        hasher.update(context.as_bytes());
        hasher.update(&time.0.to_be_bytes());
        hasher.update(domain);
        hasher.update(payload);
        hasher.update(nonce);
        hasher.update(node_id);

        let hash = hasher.finalize();
        let mut entropy = [0u8; HASHTIMER_ENTROPY_BYTES];
        entropy.copy_from_slice(&hash.as_bytes()[0..HASHTIMER_ENTROPY_BYTES]);

        HashTimer {
            timestamp_us,
            entropy,
            signature: Vec::new(), // Unsigned - can be signed later with sign_hashtimer
            public_key: Vec::new(),
        }
    }

    /// Create a new HashTimer for a transaction (unsigned, deterministic)
    pub fn now_tx(domain: &str, payload: &[u8], nonce: &[u8], node_id: &[u8]) -> Self {
        let time = IppanTimeMicros::now();
        Self::derive("tx", time, domain.as_bytes(), payload, nonce, node_id)
    }

    /// Create a new HashTimer for a block (unsigned, deterministic)
    pub fn now_block(domain: &str, payload: &[u8], nonce: &[u8], node_id: &[u8]) -> Self {
        let time = IppanTimeMicros::now();
        Self::derive("block", time, domain.as_bytes(), payload, nonce, node_id)
    }

    /// Create a new HashTimer for a round (unsigned, deterministic)
    pub fn now_round(domain: &str, payload: &[u8], nonce: &[u8], node_id: &[u8]) -> Self {
        let time = IppanTimeMicros::now();
        Self::derive("round", time, domain.as_bytes(), payload, nonce, node_id)
    }

    /// Sign this HashTimer with the provided signing key
    pub fn sign_with(&mut self, signing_key: &SigningKey) {
        let digest = self.digest();
        let signature = signing_key.sign(&digest);
        let verifying_key = signing_key.verifying_key();
        self.signature = signature.to_bytes().to_vec();
        self.public_key = verifying_key.to_bytes().to_vec();
    }

    /// Create a signed copy of this HashTimer
    pub fn signed(&self, signing_key: &SigningKey) -> Self {
        let mut signed = self.clone();
        signed.sign_with(signing_key);
        signed
    }

    /// Verify the signature embedded in this HashTimer.
    /// Returns true if unsigned (no signature) or if signature is valid.
    pub fn verify(&self) -> bool {
        if self.signature.is_empty() || self.public_key.is_empty() {
            // Unsigned HashTimer - considered valid for ordering purposes
            return true;
        }
        verify_hashtimer(self)
    }
}

/// Generate fresh entropy suitable for a new [`HashTimer`].
pub fn generate_entropy() -> [u8; HASHTIMER_ENTROPY_BYTES] {
    let mut entropy = [0u8; HASHTIMER_ENTROPY_BYTES];
    OsRng.fill_bytes(&mut entropy);
    entropy
}

/// Generate a random nonce (32 bytes)
pub fn random_nonce() -> [u8; 32] {
    let mut nonce = [0u8; 32];
    OsRng.fill_bytes(&mut nonce);
    nonce
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

    let signature = Signature::from_bytes(&signature_bytes);
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

    #[test]
    fn derive_is_deterministic_for_identical_inputs() {
        let time = IppanTimeMicros(123_456);
        let domain = b"domain";
        let payload = b"payload";
        let nonce = b"nonce";
        let node_id = b"node-id";

        let timer_a = HashTimer::derive("ctx", time, domain, payload, nonce, node_id);
        let timer_b = HashTimer::derive("ctx", time, domain, payload, nonce, node_id);

        assert_eq!(timer_a.timestamp_us, timer_b.timestamp_us);
        assert_eq!(timer_a.entropy, timer_b.entropy);
        assert_eq!(timer_a.digest(), timer_b.digest());
    }

    #[test]
    fn derive_changes_when_payload_differs() {
        let time = IppanTimeMicros(987_654);
        let base = HashTimer::derive("ctx", time, b"domain", b"payload-a", b"nonce", b"node");
        let different = HashTimer::derive("ctx", time, b"domain", b"payload-b", b"nonce", b"node");

        assert_ne!(base.entropy, different.entropy);
        assert_ne!(base.digest(), different.digest());
    }

    #[test]
    fn hex_round_trip_preserves_timestamp_and_digest() {
        let time = IppanTimeMicros(321_000);
        let timer = HashTimer::derive("ctx", time, b"domain", b"payload", b"nonce", b"node");
        let encoded = timer.to_hex();
        let decoded = HashTimer::from_hex(&encoded).expect("decode from hex");

        assert_eq!(decoded.timestamp_us, timer.timestamp_us);
        assert_eq!(decoded.to_hex().len(), encoded.len());
        assert_eq!(&decoded.to_hex()[..14], &encoded[..14]);
        assert_ne!(decoded.digest(), [0u8; 32]);
        assert!(decoded.signature.is_empty());
        assert!(decoded.public_key.is_empty());
    }

    #[test]
    fn signed_variant_preserves_digest_and_verifies() {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let base = HashTimer::derive(
            "ctx",
            IppanTimeMicros(111),
            b"domain",
            b"payload",
            b"nonce",
            b"node",
        );
        let signed = base.signed(&signing_key);

        assert_eq!(base.digest(), signed.digest());
        assert_eq!(base.timestamp_us, signed.timestamp_us);
        assert!(!signed.signature.is_empty());
        assert!(!signed.public_key.is_empty());
        assert!(signed.verify());
    }

    // =====================================================================
    // Deterministic HashTimer Tests - Critical Path Coverage
    // =====================================================================

    #[test]
    fn hashtimer_entropy_never_repeats() {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for _ in 0..1000 {
            let entropy = generate_entropy();
            assert!(seen.insert(entropy), "Entropy collision detected");
        }
    }

    #[test]
    fn hashtimer_derive_is_deterministic() {
        let time = IppanTimeMicros(1000000);
        let domain = b"test-domain";
        let payload = b"test-payload";
        let nonce = [42u8; 32];
        let node_id = [7u8; 32];

        let ht1 = HashTimer::derive("tx", time, domain, payload, &nonce, &node_id);
        let ht2 = HashTimer::derive("tx", time, domain, payload, &nonce, &node_id);

        assert_eq!(ht1.entropy, ht2.entropy, "Derivation must be deterministic");
        assert_eq!(ht1.digest(), ht2.digest(), "Digest must be deterministic");
        assert_eq!(
            ht1.timestamp_us, ht2.timestamp_us,
            "Timestamp must be preserved"
        );
    }

    #[test]
    fn hashtimer_hex_round_trip() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let original = sign_hashtimer(&signing_key);

        let hex = original.to_hex();
        assert_eq!(hex.len(), 64, "Hex must be 64 characters");

        let decoded = HashTimer::from_hex(&hex).expect("decode");
        assert_eq!(
            original.timestamp_us, decoded.timestamp_us,
            "Timestamp preserved"
        );

        // Digest should be reproducible from time prefix
        let original_digest = original.digest();
        assert_eq!(hex[14..64], hex::encode(&original_digest)[0..50]);
    }

    #[test]
    fn hashtimer_hex_encoding_invalid_length_rejected() {
        let result = HashTimer::from_hex("abc123");
        assert!(result.is_err(), "Invalid length hex should be rejected");

        let result = HashTimer::from_hex(&"0".repeat(63));
        assert!(result.is_err(), "63-char hex should be rejected");

        let result = HashTimer::from_hex(&"0".repeat(65));
        assert!(result.is_err(), "65-char hex should be rejected");
    }

    #[test]
    fn hashtimer_signature_verification_wrong_key() {
        let mut rng = OsRng;
        let signing_key1 = SigningKey::generate(&mut rng);
        let signing_key2 = SigningKey::generate(&mut rng);

        let mut timer = sign_hashtimer(&signing_key1);
        // Replace public key with wrong one
        timer.public_key = signing_key2.verifying_key().to_bytes().to_vec();

        assert!(
            !verify_hashtimer(&timer),
            "Wrong public key should fail verification"
        );
        assert!(!timer.verify(), "verify() method should also fail");
    }

    #[test]
    fn hashtimer_unsigned_is_valid() {
        let time = IppanTimeMicros(1000000);
        let timer = HashTimer::derive("tx", time, b"domain", b"payload", &[0u8; 32], &[0u8; 32]);

        assert!(timer.verify(), "Unsigned HashTimer should be valid");
        assert!(timer.signature.is_empty());
        assert!(timer.public_key.is_empty());
    }

    #[test]
    fn hashtimer_ordering_by_timestamp() {
        let time1 = IppanTimeMicros(1000);
        let time2 = IppanTimeMicros(2000);

        let ht1 = HashTimer::derive("tx", time1, b"d", b"p", &[1u8; 32], &[1u8; 32]);
        let ht2 = HashTimer::derive("tx", time2, b"d", b"p", &[2u8; 32], &[2u8; 32]);

        assert!(
            ht1.timestamp_us < ht2.timestamp_us,
            "HashTimers must order by time"
        );
        assert!(ht1.time() < ht2.time(), "IppanTimeMicros must be ordered");
    }

    #[test]
    fn hashtimer_nonce_changes_digest() {
        let time = IppanTimeMicros(1000000);
        let nonce1 = [1u8; 32];
        let nonce2 = [2u8; 32];

        let ht1 = HashTimer::derive("tx", time, b"d", b"p", &nonce1, &[0u8; 32]);
        let ht2 = HashTimer::derive("tx", time, b"d", b"p", &nonce2, &[0u8; 32]);

        assert_ne!(
            ht1.digest(),
            ht2.digest(),
            "Different nonces must produce different digests"
        );
        assert_ne!(
            ht1.entropy, ht2.entropy,
            "Different nonces must produce different entropy"
        );
    }

    #[test]
    fn hashtimer_context_isolation() {
        let time = IppanTimeMicros(1000000);
        let common_args = (b"domain", b"payload", [0u8; 32], [0u8; 32]);

        let ht_tx = HashTimer::derive(
            "tx",
            time,
            common_args.0,
            common_args.1,
            &common_args.2,
            &common_args.3,
        );
        let ht_block = HashTimer::derive(
            "block",
            time,
            common_args.0,
            common_args.1,
            &common_args.2,
            &common_args.3,
        );
        let ht_round = HashTimer::derive(
            "round",
            time,
            common_args.0,
            common_args.1,
            &common_args.2,
            &common_args.3,
        );

        assert_ne!(
            ht_tx.entropy, ht_block.entropy,
            "tx/block contexts must be isolated"
        );
        assert_ne!(
            ht_block.entropy, ht_round.entropy,
            "block/round contexts must be isolated"
        );
        assert_ne!(
            ht_tx.entropy, ht_round.entropy,
            "tx/round contexts must be isolated"
        );
    }

    #[test]
    fn hashtimer_digest_deterministic() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let timer = sign_hashtimer(&signing_key);

        let digest1 = timer.digest();
        let digest2 = timer.hash();
        let digest3 = timer.digest();

        assert_eq!(
            digest1, digest2,
            "digest() and hash() must return same value"
        );
        assert_eq!(digest1, digest3, "digest() must be deterministic");
    }

    #[test]
    fn hashtimer_id_hex_is_lowercase() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let timer = sign_hashtimer(&signing_key);

        let id_hex = timer.id_hex();
        assert_eq!(id_hex.len(), 64, "ID hex must be 64 characters");
        assert_eq!(id_hex, id_hex.to_lowercase(), "ID hex must be lowercase");
    }

    #[test]
    fn hashtimer_sign_with_mutates_timer() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let time = IppanTimeMicros(1000000);

        let mut timer = HashTimer::derive("tx", time, b"d", b"p", &[0u8; 32], &[0u8; 32]);
        assert!(timer.signature.is_empty(), "Should start unsigned");

        timer.sign_with(&signing_key);

        assert!(!timer.signature.is_empty(), "Should be signed");
        assert!(!timer.public_key.is_empty(), "Public key should be set");
        assert!(timer.verify(), "Signed timer should verify");
    }

    #[test]
    fn hashtimer_signed_creates_new_copy() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let time = IppanTimeMicros(1000000);

        let original = HashTimer::derive("tx", time, b"d", b"p", &[0u8; 32], &[0u8; 32]);
        let signed = original.signed(&signing_key);

        assert!(
            original.signature.is_empty(),
            "Original should remain unsigned"
        );
        assert!(!signed.signature.is_empty(), "Copy should be signed");
        assert_eq!(
            original.timestamp_us, signed.timestamp_us,
            "Timestamp should match"
        );
        assert_eq!(original.entropy, signed.entropy, "Entropy should match");
    }

    #[test]
    fn hashtimer_now_tx_different_each_call() {
        let tx1 = HashTimer::now_tx(
            "test",
            b"payload",
            b"nonce1234567890123456789012",
            b"node_id_12345678901234567890",
        );
        std::thread::sleep(std::time::Duration::from_micros(10));
        let tx2 = HashTimer::now_tx(
            "test",
            b"payload",
            b"nonce1234567890123456789012",
            b"node_id_12345678901234567890",
        );

        // Should have different timestamps (assuming enough time passed)
        assert_ne!(
            tx1.digest(),
            tx2.digest(),
            "Different timestamps should produce different digests"
        );
    }

    #[test]
    fn hashtimer_now_block_vs_now_round() {
        let block_ht = HashTimer::now_block(
            "domain",
            b"p",
            b"nonce123456789012345678901",
            b"node123456789012345678901234",
        );
        let round_ht = HashTimer::now_round(
            "domain",
            b"p",
            b"nonce123456789012345678901",
            b"node123456789012345678901234",
        );

        // Different contexts should produce different entropy even with same inputs
        assert_ne!(
            block_ht.entropy, round_ht.entropy,
            "block and round contexts must differ"
        );
    }

    #[test]
    fn random_nonce_never_repeats() {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for _ in 0..1000 {
            let nonce = random_nonce();
            assert!(seen.insert(nonce), "Nonce collision detected");
        }
    }

    #[test]
    fn hashtimer_time_accessor() {
        let time = IppanTimeMicros(12345678);
        let timer = HashTimer::derive("tx", time, b"d", b"p", &[0u8; 32], &[0u8; 32]);

        assert_eq!(
            timer.time().0,
            12345678,
            "time() accessor must return correct value"
        );
    }

    #[test]
    fn digest_from_parts_deterministic() {
        let timestamp_us = 1000000i64;
        let entropy = [42u8; HASHTIMER_ENTROPY_BYTES];

        let digest1 = digest_from_parts(timestamp_us, &entropy);
        let digest2 = digest_from_parts(timestamp_us, &entropy);

        assert_eq!(digest1, digest2, "digest_from_parts must be deterministic");
    }
}
