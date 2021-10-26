use std::collections::HashMap;

use crate::{Block, BlockHash, BlockHeader, BlockLocatorObject};

/// Represents a node of the tree, which is an implementation detail of the block tree, so it's not
/// part of the API.
struct Entry {
    block_header: BlockHeader,
    // Distance to the genesis block.
    height: u32,
}

/// The ledger of all transactions, which everyone in the LearnCoin network accepts as the
/// authoritative record of ownership.
/// Block Tree is a tree of blocks, with the genesis block as a root. Any path from the root to a
/// leaf is a blockchain.
pub struct BlockIndex {
    // Blocks that have a parent in the network, indexed by their hash.
    tree: HashMap<BlockHash, Entry>,
}

impl BlockIndex {
    pub fn new(genesis_block: Block) -> Self {
        let mut tree = HashMap::new();
        let genesis_hash = genesis_block.header().hash();
        tree.insert(
            genesis_hash,
            Entry {
                block_header: genesis_block.header().clone(),
                height: 0,
            },
        );
        Self { tree }
    }

    /// Returns the height for the given block hash.
    pub fn height(&self, hash: &BlockHash) -> Option<u32> {
        self.tree.get(hash).map(|entry| entry.height)
    }

    /// Returns whether or not the given block hash exists in the tree.
    pub fn exists(&self, block_hash: &BlockHash) -> bool {
        self.tree.contains_key(block_hash)
    }

    /// Adds a new block to the blockchain and updates the active blockchain if needed.
    ///
    /// Preconditions:
    ///   - The parent exists.
    pub fn insert(&mut self, block_header: BlockHeader) {
        let parent_hash = block_header.previous_block_hash();
        let parent = self.tree.get(&parent_hash).unwrap();
        let block_height = parent.height + 1;
        let previous = self.tree.insert(
            block_header.hash(),
            Entry {
                block_header,
                height: block_height,
            },
        );
        assert!(previous.is_none());
    }

    /// Returns the ancestor of the given block hash at the given height or
    /// None if the given block hash doesn't exist in the tree.
    ///
    /// Preconditions:
    ///   - Height is less than or equal to the height of the given block hash.
    pub fn ancestor(&self, block_hash: &BlockHash, height: u32) -> Option<BlockHash> {
        match self.tree.get(&block_hash) {
            None => None,
            Some(current) => {
                assert!(height <= current.height);
                if current.height == height {
                    Some(*block_hash)
                } else {
                    self.ancestor(&current.block_header.previous_block_hash(), height)
                }
            }
        }
    }

    /// Returns the block locator object.
    ///
    /// Preconditions:
    ///   - block_hash must exist
    pub fn locator(&self, block_hash: &BlockHash) -> BlockLocatorObject {
        let entry = self.tree.get(block_hash);
        assert!(entry.is_some());
        let entry = entry.unwrap();

        let mut hashes = vec![];

        let mut height = entry.height;
        let mut step = 1;
        loop {
            // Ancestor must always exist because all blocks have parents,
            // and the given block hash exists at this moment.
            hashes.push(self.ancestor(block_hash, height).unwrap());

            if height == 0 {
                // Genesis block has been inserted.
                break;
            }

            if hashes.len() >= 10 {
                step *= 2;
            }

            if step >= height {
                // Ensure we don't skip the genesis block.
                height = 0;
            } else {
                height -= step;
            }
        }
        BlockLocatorObject::new(hashes)
    }
}
