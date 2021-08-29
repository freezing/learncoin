use crate::core::{Block, BlockTree, BlockValidator, OrphanedBlocks};

/// Responsible for processing new blocks and new transactions from the network.
/// It validates that blocks and transactions are valid.
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

    /// Called when a new block is received from the network.
    /// Note that the same block may be received multiple times (e.g. each node may send the same
    /// block once, or there may be malicious nodes).
    pub fn on_block_received(&mut self, block: Block, current_time: u32) {
        if self.exists(&block) {
            // Drop the block because it already exists.
            return;
        }

        // Drop the block if context-free validation fails.
        match BlockValidator::validate_no_context(&block, current_time) {
            Ok(()) => {}
            Err(e) => {
                // Block is invalid, log a warning and drop it.
                eprintln!("{}", e);
                return;
            }
        }

        if self.block_tree.exists(block.header().previous_block_hash()) {
            // If the parent exists, validate the node and insert it
            match self.validate_and_insert_in_blocktree(block, current_time) {
                Ok(()) => {}
                Err(e) => {
                    // Validation failed, log a warning and drop the block.
                    eprintln!("{}", e);
                    return;
                }
            }
        } else {
            // If there is no parent in the block tree, the received node is orphaned.
            self.orphaned_blocks.insert(block);
        }
    }

    fn validate_and_insert_in_blocktree(
        &mut self,
        block: Block,
        current_time: u32,
    ) -> Result<(), String> {
        let chain_context = self.fetch_chain_context(&block);
        let utxo_context = self.fetch_utxo_context(&block);

        BlockValidator::validate_chain_context(&block, &chain_context, current_time)?;
        BlockValidator::validate_utxo_context(&block, &utxo_context)
    }

    fn exists(&self, block: &Block) -> bool {
        self.orphaned_blocks.exists(block) || self.block_tree.exists(&block.header().hash())
    }
}
