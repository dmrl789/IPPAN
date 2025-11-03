use crate::hashtimer::HashTimer;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Block {
    pub id: String,
    pub parent: Option<String>,
    pub timestamp: HashTimer,
    pub data: Vec<u8>,
}

#[derive(Default)]
pub struct BlockDAG {
    pub blocks: HashMap<String, Block>,
}

impl BlockDAG {
    pub fn insert(&mut self, block: Block) {
        self.blocks.insert(block.id.clone(), block);
    }

    pub fn pending(&self) -> Vec<Block> {
        self.blocks.values().cloned().collect()
    }

    pub fn finalize_round(&mut self, _time: HashTimer) {
        // Future: prune, compress, or anchor finalized blocks
    }
}
