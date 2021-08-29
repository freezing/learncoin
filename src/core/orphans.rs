use crate::core::block::BlockHash;
use crate::core::Block;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

// Blocks without a parent in the network.
// E.g. this may happen when two blocks are mined quickly one after the other,
// and the child arrives before the parent.
// Orphaned blocks are indexed by their parent hash.
pub struct Orphans {
    orphaned_blocks: HashMap<BlockHash, Vec<Block>>,
}

impl Orphans {
    pub fn new() -> Self {
        Self {
            orphaned_blocks: HashMap::new(),
        }
    }

    pub fn insert(&mut self, block: Block) {
        match self
            .orphaned_blocks
            .entry(*block.header().previous_block_hash())
        {
            Entry::Occupied(mut e) => e.get_mut().push(block),
            Entry::Vacant(e) => {
                e.insert(vec![block]);
            }
        }
    }

    pub fn remove(&mut self, parent_hash: BlockHash) -> Vec<Block> {
        self.orphaned_blocks
            .remove(&parent_hash)
            .unwrap_or_else(|| vec![])
    }
}
