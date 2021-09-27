use crate::{Block, BlockHash};
use std::collections::hash_map::Entry;
use std::collections::HashMap;

// Blocks without a parent in the local node's view.
// E.g. this may happen when two blocks are mined quickly one after the other,
// and the child arrives before the parent.
pub struct OrphanBlocks {
    // Orphaned blocks indexed by their parent hash.
    orphaned_blocks: HashMap<BlockHash, Vec<Block>>,
}

impl OrphanBlocks {
    pub fn new() -> Self {
        Self {
            orphaned_blocks: HashMap::new(),
        }
    }

    /// Returns a copy of all orphan blocks in no particular order.
    pub fn all_blocks(&self) -> Vec<Block> {
        let mut all_blocks = vec![];
        for (_, blocks) in &self.orphaned_blocks {
            for block in blocks {
                all_blocks.push(block.clone());
            }
        }
        all_blocks
    }

    /// Inserts the orphan block.
    /// If the block with the same hash already exists, this function has no effect.
    pub fn insert(&mut self, block: Block) {
        match self
            .orphaned_blocks
            .entry(block.header().previous_block_hash())
        {
            Entry::Occupied(mut e) => e.get_mut().push(block),
            Entry::Vacant(e) => {
                e.insert(vec![block]);
            }
        }
    }

    /// Returns whether or not the given block exists.
    pub fn exists(&self, target_block_hash: &BlockHash) -> bool {
        self.orphaned_blocks.iter().any(|(_, blocks)| {
            blocks
                .iter()
                .any(|existing| existing.header().hash() == *target_block_hash)
        })
    }

    /// Removes all orphan blocks with the given parent's block hash, and returns them.
    pub fn remove(&mut self, parent_hash: &BlockHash) -> Vec<Block> {
        self.orphaned_blocks
            .remove(parent_hash)
            .unwrap_or_else(|| vec![])
    }
}
