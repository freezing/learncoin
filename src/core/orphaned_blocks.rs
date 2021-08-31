use crate::core::block::BlockHash;
use crate::core::Block;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

// Blocks without a parent in the network.
// E.g. this may happen when two blocks are mined quickly one after the other,
// and the child arrives before the parent.
pub struct OrphanedBlocks {
    // Orphaned blocks indexed by their parent hash.
    orphaned_blocks: HashMap<BlockHash, Vec<Block>>,
}

impl OrphanedBlocks {
    pub fn new() -> Self {
        Self {
            orphaned_blocks: HashMap::new(),
        }
    }

    /// Inserts the block.
    /// If the block with the same hash already exists, this function has no effect.
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

    /// Returns whether or not the given block exists.
    pub fn exists(&self, block: &Block) -> bool {
        match self
            .orphaned_blocks
            .get(block.header().previous_block_hash())
        {
            None => false,
            Some(existing_blocks) => existing_blocks
                .iter()
                .any(|existing| existing.header().hash() == block.header().hash()),
        }
    }

    /// Removes all children for the given parent hash.
    pub fn remove(&mut self, parent_hash: &BlockHash) -> Vec<Block> {
        self.orphaned_blocks
            .remove(parent_hash)
            .unwrap_or_else(|| vec![])
    }
}
