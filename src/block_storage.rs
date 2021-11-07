use crate::{Block, BlockHash};
use std::collections::HashMap;

pub struct BlockStorage {
    blocks: HashMap<BlockHash, Block>,
}

impl BlockStorage {
    pub fn new(genesis_block: Block) -> Self {
        let mut blocks = HashMap::new();
        blocks.insert(*genesis_block.id(), genesis_block);
        Self { blocks }
    }

    pub fn exists(&self, block_hash: &BlockHash) -> bool {
        self.blocks.contains_key(block_hash)
    }

    pub fn insert(&mut self, block: Block) {
        self.blocks.insert(*block.id(), block);
    }

    pub fn get(&self, block_hash: &BlockHash) -> Option<&Block> {
        self.blocks.get(block_hash)
    }
}
