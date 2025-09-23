use blake3::Hasher as Blake3;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::fmt;

/// IPPAN Time: microsecond precision timestamp
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct IppanTimeMicros(pub u64);

impl IppanTimeMicros {
    /// Get current IPPAN time in microseconds
    pub fn now() -> Self {
        // This will be replaced by the proper IPPAN Time service
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        Self(now.as_micros() as u64)
    }
}

/// HashTimer: 256-bit structure with 14 hex prefix (56 bits) + 50 hex suffix (200 bits)
/// Format: `<14-hex time prefix><50-hex blake3 hash>`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HashTimer {
    /// 14 hex characters (56 bits) - microsecond IPPAN Time
    pub time_prefix: [u8; 7], // 7 bytes = 56 bits
    /// 50 hex characters (200 bits) - blake3 hash
    pub hash_suffix: [u8; 25], // 25 bytes = 200 bits
}

impl HashTimer {
    /// Create a new HashTimer for a transaction
    pub fn now_tx(domain: &str, payload: &[u8], nonce: &[u8], node_id: &[u8]) -> Self {
        let time = IppanTimeMicros::now();
        Self::derive("tx", time, domain.as_bytes(), payload, nonce, node_id)
    }

    /// Create a new HashTimer for a block
    pub fn now_block(domain: &str, payload: &[u8], nonce: &[u8], node_id: &[u8]) -> Self {
        let time = IppanTimeMicros::now();
        Self::derive("block", time, domain.as_bytes(), payload, nonce, node_id)
    }

    /// Create a new HashTimer for a round
    pub fn now_round(domain: &str, payload: &[u8], nonce: &[u8], node_id: &[u8]) -> Self {
        let time = IppanTimeMicros::now();
        Self::derive("round", time, domain.as_bytes(), payload, nonce, node_id)
    }

    /// Derive HashTimer from components
    pub fn derive(
        context: &str,
        time: IppanTimeMicros,
        domain: &[u8],
        payload: &[u8],
        nonce: &[u8],
        node_id: &[u8],
    ) -> Self {
        // Create 14-hex time prefix (56 bits)
        let time_prefix = Self::time_to_prefix(time);

        // Create 50-hex hash suffix (200 bits) using blake3
        let hash_suffix = Self::compute_hash(context, time, domain, payload, nonce, node_id);

        Self {
            time_prefix,
            hash_suffix,
        }
    }

    /// Convert time to 14-hex prefix (56 bits)
    fn time_to_prefix(time: IppanTimeMicros) -> [u8; 7] {
        let mut prefix = [0u8; 7];
        // Take the lower 56 bits of the time
        let time_bits = time.0 & 0x00FFFFFFFFFFFFFF; // Mask to 56 bits
        prefix[0..7].copy_from_slice(&time_bits.to_be_bytes()[1..8]);
        prefix
    }

    /// Compute 50-hex hash suffix using blake3
    fn compute_hash(
        context: &str,
        time: IppanTimeMicros,
        domain: &[u8],
        payload: &[u8],
        nonce: &[u8],
        node_id: &[u8],
    ) -> [u8; 25] {
        let mut hasher = Blake3::new();
        hasher.update(context.as_bytes());
        hasher.update(&time.0.to_be_bytes());
        hasher.update(domain);
        hasher.update(payload);
        hasher.update(nonce);
        hasher.update(node_id);

        let hash = hasher.finalize();
        let mut suffix = [0u8; 25];
        // Take first 25 bytes (200 bits) of the blake3 hash
        suffix.copy_from_slice(&hash.as_bytes()[0..25]);
        suffix
    }

    /// Convert to hex string representation
    pub fn to_hex(&self) -> String {
        let time_hex = hex::encode(self.time_prefix);
        let hash_hex = hex::encode(self.hash_suffix);
        format!("{}{}", time_hex, hash_hex)
    }

    /// Parse from hex string
    pub fn from_hex(hex_str: &str) -> Result<Self, String> {
        if hex_str.len() != 64 {
            return Err(format!(
                "HashTimer hex must be 64 characters, got {}",
                hex_str.len()
            ));
        }

        let time_hex = &hex_str[0..14];
        let hash_hex = &hex_str[14..64];

        let time_prefix = hex::decode(time_hex)
            .map_err(|e| format!("Invalid time prefix hex: {}", e))?
            .try_into()
            .map_err(|_| "Time prefix must be 7 bytes")?;

        let hash_suffix = hex::decode(hash_hex)
            .map_err(|e| format!("Invalid hash suffix hex: {}", e))?
            .try_into()
            .map_err(|_| "Hash suffix must be 25 bytes")?;

        Ok(Self {
            time_prefix,
            hash_suffix,
        })
    }

    /// Get the time component from the prefix
    pub fn time(&self) -> IppanTimeMicros {
        let mut time_bytes = [0u8; 8];
        time_bytes[1..8].copy_from_slice(&self.time_prefix);
        IppanTimeMicros(u64::from_be_bytes(time_bytes))
    }
}

impl fmt::Display for HashTimer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Generate a random nonce
pub fn random_nonce() -> [u8; 32] {
    let mut nonce = [0u8; 32];
    OsRng.fill_bytes(&mut nonce);
    nonce
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hashtimer_roundtrip() {
        let domain = "test_domain";
        let payload = b"test_payload";
        let nonce = random_nonce();
        let node_id = b"test_node_id";

        let ht1 = HashTimer::derive(
            "test",
            IppanTimeMicros(1234567890123456),
            domain.as_bytes(),
            payload,
            &nonce,
            node_id,
        );
        let hex_str = ht1.to_hex();
        let ht2 = HashTimer::from_hex(&hex_str).unwrap();

        assert_eq!(ht1, ht2);
        assert_eq!(hex_str.len(), 64);
        assert_eq!(hex_str[0..14].len(), 14); // time prefix
        assert_eq!(hex_str[14..64].len(), 50); // hash suffix
    }

    #[test]
    fn test_hashtimer_determinism() {
        let domain = "test_domain";
        let payload = b"test_payload";
        let nonce = random_nonce();
        let node_id = b"test_node_id";
        let time = IppanTimeMicros(1234567890123456);

        let ht1 = HashTimer::derive("test", time, domain.as_bytes(), payload, &nonce, node_id);
        let ht2 = HashTimer::derive("test", time, domain.as_bytes(), payload, &nonce, node_id);

        assert_eq!(ht1, ht2);
        assert_eq!(ht1.to_hex(), ht2.to_hex());
    }

    #[test]
    fn test_hashtimer_different_contexts() {
        let domain = "test_domain";
        let payload = b"test_payload";
        let nonce = random_nonce();
        let node_id = b"test_node_id";
        let time = IppanTimeMicros(1234567890123456);

        let ht_tx = HashTimer::derive("tx", time, domain.as_bytes(), payload, &nonce, node_id);
        let ht_block =
            HashTimer::derive("block", time, domain.as_bytes(), payload, &nonce, node_id);

        assert_ne!(ht_tx, ht_block);
        assert_ne!(ht_tx.to_hex(), ht_block.to_hex());
    }

    #[test]
    fn test_time_extraction() {
        let original_time = IppanTimeMicros(1234567890123456);
        let domain = "test";
        let payload = b"test";
        let nonce = random_nonce();
        let node_id = b"test";

        let ht = HashTimer::derive(
            "test",
            original_time,
            domain.as_bytes(),
            payload,
            &nonce,
            node_id,
        );
        let extracted_time = ht.time();

        // Should be close to original (within 56-bit precision)
        assert_eq!(
            original_time.0 & 0x00FFFFFFFFFFFFFF,
            extracted_time.0 & 0x00FFFFFFFFFFFFFF
        );
    }
}
