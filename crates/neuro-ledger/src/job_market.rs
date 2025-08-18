use neuro_core::*;
use sled::Db;
use bincode;

const JOBS_CF: &str = "jobs";
const BIDS_CF: &str = "bids";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Bid {
    pub job_id: Hash,
    pub executor_id: Address,
    pub price_ipn: u128,
    pub est_latency_ms: u32,
    pub tee: bool,
}

pub struct JobMarket { db: Db }

impl JobMarket {
    pub fn open(path: &str) -> Self { Self { db: sled::open(path).unwrap() } }

    pub fn post_job(&self, job: &InferenceJob) -> anyhow::Result<()> {
        self.db.open_tree(JOBS_CF)?.insert(job.id, bincode::serialize(job)?)?;
        Ok(())
    }

    pub fn place_bid(&self, bid: &Bid) -> anyhow::Result<()> {
        let tree = self.db.open_tree(BIDS_CF)?;
        let mut key = Vec::from(bid.job_id);
        key.extend_from_slice(&bid.executor_id);
        tree.insert(key, bincode::serialize(bid)?)?;
        Ok(())
    }

    pub fn select_winner(&self, job_id: &Hash) -> anyhow::Result<Option<Bid>> {
        let bids_tree = self.db.open_tree(BIDS_CF)?;
        let mut best: Option<Bid> = None;
        for item in bids_tree.iter() {
            let (_, v) = item?;
            let b: Bid = bincode::deserialize(&v)?;
            if &b.job_id != job_id { continue; }
            best = Some(match best {
                None => b,
                Some(prev) => {
                    if b.price_ipn < prev.price_ipn ||
                       (b.price_ipn == prev.price_ipn && b.est_latency_ms < prev.est_latency_ms) { b } else { prev }
                }
            });
        }
        Ok(best)
    }
}
