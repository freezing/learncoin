use crate::core::{
    Block, BlockTree, BlockValidator, ChainContext, OrphanedBlocks, Transaction, TransactionPool,
    UtxoContext, UtxoPool,
};

/// Responsible for processing new blocks and new transactions from the network.
/// It validates that blocks and transactions are valid.
pub struct BlockchainManager {
    block_tree: BlockTree,
    orphaned_blocks: OrphanedBlocks,
    transaction_pool: TransactionPool,
    utxo_pool: UtxoPool,
}

impl BlockchainManager {
    pub fn new() -> Self {
        Self {
            block_tree: BlockTree::new(),
            orphaned_blocks: OrphanedBlocks::new(),
            transaction_pool: TransactionPool::new(),
            utxo_pool: UtxoPool::new(),
        }
    }

    /// Called when a new block is received from the network.
    /// Note that the same block may be received multiple times (e.g. each node may send the same
    /// block once, or there may be malicious nodes).
    pub fn on_block_received(&mut self, block: Block, current_time: u32) -> Result<(), String> {
        if self.exists(&block) {
            // Drop the block because it already exists.
            return Ok(());
        }

        // Drop the block if context-free validation fails.
        BlockValidator::validate_no_context(&block, current_time)?;

        if self.block_tree.exists(block.header().previous_block_hash()) {
            // If the parent exists, validate the node and insert it
            self.validate_and_insert_in_blocktree(block, current_time)
        } else {
            // If there is no parent in the block tree, the received node is orphaned.
            self.orphaned_blocks.insert(block);
            Ok(())
        }
    }

    /// Called when a new transaction is recevied from the network.
    /// Note that the same transaction may be received multiple times.
    pub fn on_transaction_received(
        &mut self,
        transaction: Transaction,
        _current_time: u32,
    ) -> Result<(), String> {
        self.transaction_pool.insert(transaction);
        Ok(())
    }

    fn validate_and_insert_in_blocktree(
        &mut self,
        block: Block,
        current_time: u32,
    ) -> Result<(), String> {
        let chain_context = self.fetch_chain_context(&block);
        let utxo_context = self.fetch_utxo_context(&block);

        BlockValidator::validate_chain_context(&block, &chain_context, current_time)?;
        BlockValidator::validate_utxo_context(&block, &utxo_context)?;

        let block_hash = block.header().hash();
        self.block_tree.insert(block);
        // At this point, it is possible that some orphaned blocks have a new parent in the
        // block tree.
        // Make sure that we insert them.
        let orphaned_blocks = self.orphaned_blocks.remove(block_hash);

        let mut errors = vec![];
        for orphaned_block in orphaned_blocks {
            // It's important that we do not return on the first error, because some blocks may
            // be valid and should be processed in that case.
            match self.validate_and_insert_in_blocktree(orphaned_block, current_time) {
                Ok(()) => {}
                Err(e) => errors.push(e),
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n"))
        }
    }

    fn fetch_chain_context(&self, _block: &Block) -> ChainContext {
        todo!()
    }

    fn fetch_utxo_context(&self, _block: &Block) -> UtxoContext {
        todo!()
    }

    fn exists(&self, block: &Block) -> bool {
        self.orphaned_blocks.exists(block) || self.block_tree.exists(&block.header().hash())
    }
}
