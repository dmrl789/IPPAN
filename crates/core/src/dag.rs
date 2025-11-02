//! Persistent BlockDAG storage built on sled.
//!
//! The `BlockDAG` type stores verified [`Block`](crate::block::Block)
//! instances in a sled database, tracks the current tip set, and exposes
//! lightweight helpers used by the P2P synchronization service.

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use serde_json::{from_slice, to_vec};
use sled::{Config, Db, Tree};

use crate::block::Block;

const BLOCK_TREE: &str = "blocks";
const TIP_TREE: &str = "tips";

/// Sled-backed BlockDAG store.
#[derive(Clone)]
pub struct BlockDAG {
    db: Db,
    blocks: Tree,
    tips: Tree,
}

impl BlockDAG {
    /// Open (or create) a BlockDAG at the provided filesystem path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = Config::default()
            .path(path)
            .create_new(false)
            .mode(sled::Mode::HighThroughput)
            .open()
            .context("failed to open sled database for BlockDAG")?;

        let blocks = db
            .open_tree(BLOCK_TREE)
            .context("failed to open BlockDAG block tree")?;
        let tips = db
            .open_tree(TIP_TREE)
            .context("failed to open BlockDAG tip tree")?;

        Ok(Self { db, blocks, tips })
    }

    /// Insert a verified block. Returns `true` when the block was new.
    pub fn insert_block(&self, block: &Block) -> Result<bool> {
        if !block.verify() {
            return Err(anyhow!(
                "attempted to insert block that failed verification"
            ));
        }

        let hash = block.hash();
        if self.blocks.contains_key(hash.as_slice())? {
            return Ok(false);
        }

        let encoded = to_vec(block).context("failed to serialize block")?;
        self.blocks
            .insert(hash.as_slice(), encoded)
            .context("failed to persist block in sled")?;

        for parent in &block.header.parent_hashes {
            self.tips
                .remove(parent)
                .context("failed to prune parent from tip set")?;
        }
        self.tips
            .insert(hash.as_slice(), &[] as &[u8])
            .context("failed to update tip set")?;

        Ok(true)
    }

    /// Retrieve a block by hash.
    pub fn get_block(&self, hash: &[u8; 32]) -> Result<Option<Block>> {
        let data = match self.blocks.get(hash.as_slice())? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };
        let block = from_slice(&data).context("failed to deserialize stored block")?;
        Ok(Some(block))
    }

    /// Return the current tip hashes.
    pub fn get_tips(&self) -> Result<Vec<[u8; 32]>> {
        let mut tips = Vec::new();
        for entry in self.tips.iter() {
            let (key, _) = entry?;
            if key.len() == 32 {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&key);
                tips.push(hash);
            }
        }
        Ok(tips)
    }

    /// Determine whether the DAG already contains `hash`.
    pub fn contains(&self, hash: &[u8; 32]) -> Result<bool> {
        Ok(self.blocks.contains_key(hash.as_slice())?)
    }

    /// Flush pending writes to disk.
    pub fn flush(&self) -> Result<()> {
        self.db
            .flush()
            .context("failed to flush BlockDAG database")?;
        Ok(())
    }

    /// Get all block hashes in the DAG
    pub fn get_all_blocks(&self) -> Result<Vec<[u8; 32]>> {
        let mut block_hashes = Vec::new();
        for result in self.blocks.iter() {
            let (key, _) = result.context("failed to read block from database")?;
            if key.len() == 32 {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&key);
                block_hashes.push(hash);
            }
        }
        Ok(block_hashes)
    }
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand_core::OsRng;
    use tempfile::tempdir;

    use crate::block::Block;

    #[test]
    fn inserts_and_reads_blocks() {
        let dir = tempdir().unwrap();
        let dag = BlockDAG::open(dir.path()).unwrap();

        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let block = Block::new(&signing_key, vec![], vec![b"hello".to_vec()]);
        let hash = block.hash();

        assert!(dag.insert_block(&block).unwrap());
        assert!(dag.contains(&hash).unwrap());

        let stored = dag.get_block(&hash).unwrap().expect("block missing");
        assert_eq!(stored.hash(), hash);
        assert_eq!(stored.header.parent_hashes.len(), 0);

        let tips = dag.get_tips().unwrap();
        assert_eq!(tips, vec![hash]);
    }

    #[test]
    fn prevents_duplicate_insertions() {
        let dir = tempdir().unwrap();
        let dag = BlockDAG::open(dir.path()).unwrap();

        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let block = Block::new(&signing_key, vec![], vec![b"hello".to_vec()]);

        assert!(dag.insert_block(&block).unwrap());
        assert!(!dag.insert_block(&block).unwrap());
    }
}
