use sled::Db;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Reputation {
    pub uptime_ms: u64,
    pub jobs_ok: u64,
    pub jobs_late: u64,
    pub slash_points: u64,
}

pub struct ReputationLedger { db: Db }

impl ReputationLedger {
    pub fn open(path: &str) -> Self { Self { db: sled::open(path).unwrap() } }
}
