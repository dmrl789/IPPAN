use neuro_core::*;
use sled::Db;

pub struct RoyaltyRouter { db: Db }

impl RoyaltyRouter {
    pub fn open(path: &str) -> Self { Self { db: sled::open(path).unwrap() } }

    pub fn settle_inference(&self, _job: &InferenceJob, _exec: &Address, _amount_ipn: u128) -> anyhow::Result<()> {
        // TODO: split to model owner, dataset owners, executor.
        Ok(())
    }
}
