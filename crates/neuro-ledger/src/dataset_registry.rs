use neuro_core::*;
use sled::Db;
use bincode;

const DATASETS_CF: &str = "datasets";

pub struct DatasetRegistry { db: Db }

impl DatasetRegistry {
    pub fn open(path: &str) -> Self { Self { db: sled::open(path).unwrap() } }

    pub fn put(&self, asset: &DatasetAsset) -> anyhow::Result<()> {
        let tree = self.db.open_tree(DATASETS_CF)?;
        tree.insert(asset.id, bincode::serialize(asset)?)?;
        Ok(())
    }

    pub fn get(&self, id: &Hash) -> anyhow::Result<Option<DatasetAsset>> {
        let tree = self.db.open_tree(DATASETS_CF)?;
        Ok(tree.get(id)?.map(|v| bincode::deserialize(&v).unwrap()))
    }
}
