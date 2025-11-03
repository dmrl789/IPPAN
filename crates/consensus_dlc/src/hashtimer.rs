use blake3::Hasher;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HashTimer {
    pub timestamp: DateTime<Utc>,
    pub hash: String,
}

impl HashTimer {
    pub fn now() -> Self {
        let now = Utc::now();
        let mut hasher = Hasher::new();
        hasher.update(now.timestamp_nanos().to_le_bytes().as_slice());
        Self {
            timestamp: now,
            hash: hasher.finalize().to_hex().to_string(),
        }
    }

    /// Deterministic ordering of events by HashTimer hash
    pub fn order(a: &Self, b: &Self) -> std::cmp::Ordering {
        a.hash.cmp(&b.hash)
    }
}
