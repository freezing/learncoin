use crate::block_tree::BlockTree;
use crate::{Block, BlockHash, BlockHeader, OrphanBlocks, Transaction, TransactionOutput};

/// Responsible for processing new blocks that arrive from the network.
/// It keeps track of all the blocks in the blockchain, including the active blockchain,
/// secondary chains and orphan blocks.
/// The Blockchain doesn't do any validation. It assumes that all the blocks are already validated.
pub struct Blockchain {
    block_tree: BlockTree,
    orphan_blocks: OrphanBlocks,
}

impl Blockchain {
    pub fn new(genesis_block: Block) -> Self {
        Self {
            block_tree: BlockTree::new(genesis_block),
            orphan_blocks: OrphanBlocks::new(),
        }
    }

    /// Returns the hash of the last block in the active blockchain.
    pub fn tip(&self) -> &BlockHash {
        self.block_tree.tip()
    }

    /// Returns a copy of all the blocks in the blockchain in no particular order.
    pub fn all_blocks(&self) -> Vec<Block> {
        let mut all_blocks = vec![];
        for block in &self.block_tree.all() {
            all_blocks.push(block.clone());
        }

        for block in &self.orphaned_blocks.all() {
            all_blocks.push(block.clone());
        }
        all_blocks
    }

    pub fn orphan_blocks(&self) -> &OrphanBlocks {
        &self.orphan_blocks
    }

    pub fn block_tree(&self) -> &BlockTree {
        &self.block_tree
    }

    /// Inserts the new block in the blockchain. It updates the active blockchain, secondary chains
    /// and orphan blocks if necessary.
    /// Returns the orphan nodes that now have a parent and deletes them from the blockchain.
    /// It is up to the user of this API to ensure the orphan nodes are inserted back.
    /// This is useful to allow the higher-level logic to run any validation checks before
    /// inserting the orphan blocks again.
    pub fn new_block(&mut self, block: Block) -> Vec<Block> {
        if self
            .block_tree
            .exists(&block.header().previous_block_hash())
        {
            let orphans = self.orphaned_blocks.remove(block.id());
            // If the parent exists, validate the node and insert it
            self.block_tree.insert(block);
            orphans
        } else {
            // If there is no parent in the block tree, the received node is orphaned.
            self.orphaned_blocks.insert(block);
            vec![]
        }
    }

    /// Returns whether or not the given block exists in the blockchain.
    pub fn exists(&self, block: &Block) -> bool {
        self.orphaned_blocks.exists(block) || self.block_tree.exists(&block.header().hash())
    }
}
