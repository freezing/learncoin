use crate::core::block::BlockHash;
use crate::core::transaction::{TransactionInput, TransactionOutput};
use crate::core::{Address, Block, BlockValidator, Coolcoin, Transaction};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

struct BlockTreeEntry {
    block: Block,
    height: u32,
}

struct ActiveBlock {
    hash: BlockHash,
    total_work: u32,
}

/// The global public ledger of all transactions, which everyone in the Coolcoin network accept
/// as the authoritative record of ownership.
/// Block Tree is a tree of blocks with the genesis block as a root.
/// Any path from root to a leaf is a blockchain.
/// The path with the most work is called the active blockchain, while the remaining paths are
/// called secondary chains.
/// The path with most work is usually the longest path, but not always
/// (e.g. difficulty may be different for a different path).
pub struct BlockTree {
    // Blocks that have a parent in the network, indexed by their hash.
    tree: HashMap<BlockHash, BlockTreeEntry>,
    // A hash of the last block in the active blockchain.
    active_block: ActiveBlock,
}

impl BlockTree {
    // TODO: Take genesis_block as parameter.
    pub fn new() -> Self {
        let genesis_block = Self::genesis_block();
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            // Bitcoin timestamp runs out in year 2106.
            .as_secs() as u32;
        BlockValidator::validate_no_context(&genesis_block, current_time).unwrap();
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
            current_entry = self.tree.get(&tree_entry.block.header().hash());
        }
        blockchain.into_iter().rev().collect()
    }

    pub fn get(&self, block_hash: &BlockHash) -> Option<&Block> {
        self.tree.get(block_hash).map(|entry| &entry.block)
    }

    /// Adds new block to the blockchain. It assumes that the block is valid and all
    /// necessary validation has been perform before calling this function.
    ///
    /// Preconditions:
    ///   - Parent exists.
    pub fn insert(&mut self, block: Block) {
        let parent_hash = block.header().previous_block_hash();
        let block_hash = block.header().hash();
        let parent = self.tree.get(parent_hash).unwrap();
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

    /// Returns the hash of the last block in the active blockchain.
    pub fn tip(&self) -> &BlockHash {
        &self.active_block.hash
    }

    /// Returns the fork, as well as paths from each node to the fork.
    /// Fork is a block which is the lowest common ancestor for the given nodes that has
    /// multiple children.
    pub fn find_fork(
        &self,
        hash_a: &BlockHash,
        hash_b: &BlockHash,
    ) -> Option<(BlockHash, Vec<BlockHash>, Vec<BlockHash>)> {
        assert_ne!(hash_a, hash_b);
        let mut path_a = vec![];
        let mut path_b = vec![];

        // Bring to the same height.
        let mut hash_a = hash_a;
        let mut hash_b = hash_b;
        loop {
            match (self.tree.get(hash_a), self.tree.get(hash_b)) {
                // If any of the nodes doesn't exist in the tree, then fork doesn't exist neither.
                (None, _) | (_, None) => return None,
                (Some(a), Some(b)) => match a.height.cmp(&b.height) {
                    Ordering::Less => {
                        path_b.push(*hash_b);
                        hash_b = b.block.header().previous_block_hash()
                    }
                    Ordering::Equal => break,
                    Ordering::Greater => {
                        path_a.push(*hash_a);
                        hash_a = a.block.header().previous_block_hash()
                    }
                },
            };
        }

        // A and B are at the same height.
        if hash_a == hash_b {
            // LCA is found.
            Some((*hash_a, path_a, path_b))
        } else {
            while hash_a != hash_b {
                match (self.tree.get(hash_a), self.tree.get(hash_b)) {
                    (None, _) | (_, None) => return None,
                    (Some(a), Some(b)) => {
                        path_a.push(*hash_a);
                        path_b.push(*hash_b);

                        hash_a = a.block.header().previous_block_hash();
                        hash_b = b.block.header().previous_block_hash();
                    }
                }
            }
            Some((*hash_a, path_a, path_b))
        }
    }

    pub fn height(&self, hash: &BlockHash) -> Option<u32> {
        self.tree.get(hash).map(|entry| entry.height)
    }

    pub fn exists(&self, block_hash: &BlockHash) -> bool {
        self.tree.contains_key(block_hash)
    }

    fn maybe_update_active_block(&mut self, block_hash: BlockHash, new_block_total_work: u32) {
        if self.active_block.total_work < new_block_total_work {
            self.active_block = ActiveBlock {
                hash: block_hash,
                total_work: new_block_total_work,
            };
        }
    }

    fn genesis_block() -> Block {
        const GENESIS_REWARD: Coolcoin = Coolcoin::new(50);
        // TODO: Generate genesis address.
        let genesis_address = Address::new([0; 64]);
        let locktime = 0;
        let inputs = vec![TransactionInput::new_coinbase()];
        let outputs = vec![TransactionOutput::new(genesis_address, GENESIS_REWARD)];
        let _transactions = vec![Transaction::new(inputs, outputs, locktime).unwrap()];

        todo!("Requires miner to be able to find the correct nonce for the genesis block.")
        // let header = BlockHeader::new(BlockHash::new());
        // Block::new(header, transactions)
    }
}
