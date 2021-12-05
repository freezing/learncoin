use std::collections::HashMap;

use crate::{Block, BlockHash, BlockHeader, BlockLocatorObject};
use std::cmp::Ordering;

/// Represents a node of the tree, which is an implementation detail of the block tree, so it's not
/// part of the API.
#[derive(Debug)]
pub struct BlockIndexNode {
    pub block_header: BlockHeader,
    // Distance to the genesis block.
    pub height: usize,
    // Total mining work required to mine the block header.
    pub chain_work: u64,
}

/// The blockchain is a tree shaped structure starting with the Genesis block at the root,
/// with each block potentially having multiple children, but only one of them is part of
/// the active chain.
pub struct BlockIndex {
    // Blocks that have a parent in the network, indexed by their hash.
    tree: HashMap<BlockHash, BlockIndexNode>,
}

impl BlockIndex {
    pub fn new(genesis_block: Block) -> Self {
        let mut tree = HashMap::new();
        let genesis_hash = genesis_block.header().hash();
        tree.insert(
            genesis_hash,
            BlockIndexNode {
                block_header: genesis_block.header().clone(),
                height: 0,
                chain_work: 0,
            },
        );
        Self { tree }
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
        let height = parent.height + 1;
        // For the time being, we approximate the total chain work via block height.
        // TODO: Implement the actual chain work.
        let chain_work = parent.chain_work + 1;
        let previous = self.tree.insert(
            block_header.hash(),
            BlockIndexNode {
                block_header,
                height,
                chain_work,
            },
        );
        assert!(previous.is_none());
    }

    pub fn get_block_index_node(&self, block_hash: &BlockHash) -> Option<&BlockIndexNode> {
        self.tree.get(block_hash)
    }

    pub fn parent(&self, block_hash: &BlockHash) -> Option<BlockHash> {
        self.tree
            .get(block_hash)
            .map(|node| node.block_header.previous_block_hash())
    }

    /// Returns the ancestor of the given block hash at the given height or
    /// None if the given block hash doesn't exist in the tree.
    ///
    /// Preconditions:
    ///   - Height is less than or equal to the height of the given block hash.
    pub fn ancestor(&self, block_hash: &BlockHash, height: usize) -> Option<&BlockIndexNode> {
        match self.tree.get(&block_hash) {
            None => None,
            Some(current) => {
                assert!(height <= current.height);
                if current.height == height {
                    Some(current)
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
            hashes.push(
                self.ancestor(block_hash, height)
                    .unwrap()
                    .block_header
                    .hash(),
            );

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

    /// Returns the fork, as well as paths from each node to the fork, excluding the fork.
    /// Fork is a block which is the lowest common ancestor for the given nodes that has
    /// multiple children.
    /// Returns None if any of the blocks doesn't exist.
    pub fn find_fork(
        &self,
        hash_a: &BlockHash,
        hash_b: &BlockHash,
    ) -> Option<(BlockHash, Vec<BlockHash>, Vec<BlockHash>)> {
        let mut path_a = vec![];
        let mut path_b = vec![];

        let mut hash_a = *hash_a;
        let mut hash_b = *hash_b;

        // Bring them to the same height.
        loop {
            match (self.tree.get(&hash_a), self.tree.get(&hash_b)) {
                // If any of the nodes doesn't exist in the tree, then fork doesn't exist neither.
                (None, _) | (_, None) => return None,
                (Some(a), Some(b)) => match a.height.cmp(&b.height) {
                    Ordering::Less => {
                        path_b.push(hash_b);
                        hash_b = b.block_header.previous_block_hash();
                    }
                    Ordering::Equal => break,
                    Ordering::Greater => {
                        path_a.push(hash_a);
                        hash_a = a.block_header.previous_block_hash();
                    }
                },
            };
        }

        // Go to the parent block, until pointers are the same.
        while hash_a != hash_b {
            match (self.tree.get(&hash_a), self.tree.get(&hash_b)) {
                (None, _) | (_, None) => return None,
                (Some(a), Some(b)) => {
                    path_a.push(hash_a);
                    path_b.push(hash_b);

                    hash_a = a.block_header.previous_block_hash();
                    hash_b = b.block_header.previous_block_hash();
                }
            }
        }
        Some((hash_a, path_a, path_b))
    }
}
