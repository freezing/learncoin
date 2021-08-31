use crate::core::block::BlockHash;
use crate::core::{
    Block, BlockTree, BlockValidator, ChainContext, OrphanedBlocks, Transaction, TransactionPool,
    UtxoContext, UtxoPool,
};

/// Responsible for processing new blocks and new transactions from the network.
/// It validates that blocks and transactions are valid.
/// TODO: Maybe can be called Blockchain?
pub struct BlockchainManager {
    block_tree: BlockTree,
    orphaned_blocks: OrphanedBlocks,
}

impl BlockchainManager {
    pub fn new() -> Self {
        Self {
            block_tree: BlockTree::new(),
            orphaned_blocks: OrphanedBlocks::new(),
        }
    }

    pub fn tip(&self) -> &BlockHash {
        self.block_tree.tip()
    }

    pub fn block_tree(&self) -> &BlockTree {
        &self.block_tree
    }

    /// Assumes that the block is valid.
    pub fn new_block(&mut self, block: Block) -> Vec<Block> {
        if self.block_tree.exists(block.header().previous_block_hash()) {
            let orphans = self
                .orphaned_blocks
                .remove(block.header().previous_block_hash());
            // If the parent exists, validate the node and insert it
            self.block_tree.insert(block);
            orphans
        } else {
            // If there is no parent in the block tree, the received node is orphaned.
            self.orphaned_blocks.insert(block);
            vec![]
        }
    }

    pub fn exists(&self, block: &Block) -> bool {
        self.orphaned_blocks.exists(block) || self.block_tree.exists(&block.header().hash())
    }
}
