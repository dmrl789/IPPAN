use neuro_core::*;
use sled::Db;
use bincode;

const PROOFS_CF: &str = "proofs";

pub struct ProofStore { db: Db }

impl ProofStore {
    pub fn open(path: &str) -> Self { Self { db: sled::open(path).unwrap() } }

    pub fn submit(&self, pr: &ProofReceipt) -> anyhow::Result<()> {
        let id = match pr {
            ProofReceipt::PoI{job_id, ..} => job_id,
            ProofReceipt::PoL{task_id, ..} => task_id,
        };
        self.db.open_tree(PROOFS_CF)?
            .insert(*id, bincode::serialize(pr)?)?;
        Ok(())
    }

    pub fn get(&self, id: &Hash) -> anyhow::Result<Option<ProofReceipt>> {
        Ok(self.db.open_tree(PROOFS_CF)?
           .get(id)?
           .map(|v| bincode::deserialize(&v).unwrap()))
    }
}
