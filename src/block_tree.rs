use crate::{Block, BlockHash};
use std::collections::HashMap;

/// Represents a node of the tree, which is an implementation detail of the block tree, so it's not
/// part of the API.
struct BlockTreeEntry {
    block: Block,
    // Distance to the genesis block.
    height: u32,
}

/// Represents metadata of the last block in the active blockchain.
struct ActiveBlock {
    hash: BlockHash,
    // Total work that the miners have done to find this block, which is required for the consensus
    // algorithm to decide which blockchain to keep when there are multiple options, e.g., when
    // two miners mine the block around the same time.
    total_work: u32,
}

/// The ledger of all transactions, which everyone in the LearnCoin network accepts as the
/// authoritative record of ownership.
/// Block Tree is a tree of blocks, with the genesis block as a root. Any path from the root to a
/// leaf is a blockchain.
/// The path with the most work is called the active blockchain, while the other paths are called
/// secondary blockchains.
/// The path with the most work is usually the longest, but not always.
/// However, this is out of scope for now. We are going to use the height as a proxy to represent
/// the total work.
pub struct BlockTree {
    // Blocks that have a parent in the network, indexed by their hash.
    tree: HashMap<BlockHash, BlockTreeEntry>,
    // Metadata of the last block in the active blockchain.
    active_block: ActiveBlock,
}

impl BlockTree {
    pub fn new(genesis_block: Block) -> Self {
        let mut tree = HashMap::new();
        let genesis_hash = genesis_block.header().hash();
        tree.insert(
            genesis_hash,
            BlockTreeEntry {
                block: genesis_block,
                height: 0,
            },
        );
        Self {
            tree,
            active_block: ActiveBlock {
                hash: genesis_hash,
                total_work: 0,
            },
        }
    }

    pub fn active_blockchain(&self) -> Vec<Block> {
        let mut blockchain = vec![];
        let mut current_entry = Some(self.tree.get(&self.active_block.hash).unwrap());
        while let Some(tree_entry) = current_entry {
            blockchain.push(tree_entry.block.clone());
            current_entry = self
                .tree
                .get(&tree_entry.block.header().previous_block_hash());
        }
        blockchain.into_iter().rev().collect()
    }

    /// Returns the list of locator hashes based on the following algorithm:
    ///   1) Include the current block hash.
    ///      The current block hash is initially the tip.
    ///   2) Skip N blocks.
    ///      N is 0 for the first 10 blocks, then it grows exponentially for each block.
    ///   3) Repeat 1) and 2) until there are no more blocks.
    ///      Ensure the genesis block is included.
    ///
    /// Based on:
    /// https://github.com/bitcoin/bitcoin/blob/7fcf53f7b4524572d1d0c9a5fdc388e87eb02416/src/chain.cpp#L23
    pub fn locator_hashes(&self) -> Vec<BlockHash> {
        let mut hashes = vec![];
        let mut current_entry = self.tree.get(&self.active_block.hash).unwrap();
        let mut step = 1;
        // Stop when we have added the genesis block.
        loop {
            hashes.push(current_entry.block.header().hash());

            if hashes.len() > 10 {
                step *= 2;
            }

            if current_entry.height == 0 {
                // Genesis block has been inserted. We're done.
                break;
            }
            // Move to the next hash and skip `step - 1` hashes or until we find the genesis block.
            for _ in 0..step {
                if current_entry.height == 0 {
                    break;
                }
                // Safety: Hash is guaranteed to exist because current_entry.height > 0.
                assert!(current_entry.height > 0);
                current_entry = self
                    .tree
                    .get(&current_entry.block.header().previous_block_hash())
                    .unwrap();
            }
        }
        hashes
    }

    /// Returns a copy of all the blocks in the block tree in no particular order.
    pub fn all_blocks(&self) -> Vec<Block> {
        self.tree.values().map(|e| e.block.clone()).collect()
    }

    /// Returns the hash of the last block in the active blockchain.
    pub fn tip(&self) -> &BlockHash {
        &self.active_block.hash
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
    pub fn insert(&mut self, block: Block) {
        let parent_hash = block.header().previous_block_hash();
        let block_hash = block.header().hash();
        let parent = self.tree.get(&parent_hash).unwrap();
        let block_height = parent.height + 1;
        let previous = self.tree.insert(
            block.header().hash(),
            BlockTreeEntry {
                block,
                height: block_height,
            },
        );
        assert!(previous.is_none());
        // For simplicity, we are using height as an approximation of total work.
        // This is usually the case in practice, but there are some corner cases when this
        // may not be true.
        self.maybe_update_active_block(block_hash, block_height);
    }

    fn maybe_update_active_block(&mut self, block_hash: BlockHash, new_block_total_work: u32) {
        if self.active_block.total_work < new_block_total_work {
            self.active_block = ActiveBlock {
                hash: block_hash,
                total_work: new_block_total_work,
            };
        }
    }
}
