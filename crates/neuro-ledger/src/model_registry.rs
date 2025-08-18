use neuro_core::*;
use sled::Db;
use bincode;

const MODELS_CF: &str = "models";

pub struct ModelRegistry { db: Db }

impl ModelRegistry {
    pub fn open(path: &str) -> Self { Self { db: sled::open(path).expect("open sled") } }

    pub fn put(&self, asset: &ModelAsset) -> anyhow::Result<()> {
        let tree = self.db.open_tree(MODELS_CF)?;
        tree.insert(asset.id, bincode::serialize(asset)?)?;
        Ok(())
    }

    pub fn get(&self, id: &Hash) -> anyhow::Result<Option<ModelAsset>> {
        let tree = self.db.open_tree(MODELS_CF)?;
        Ok(tree.get(id)?.map(|v| bincode::deserialize(&v).unwrap()))
    }
}
