use crate::core::block::{BlockHash, BlockHeader};
use crate::core::hash::merkle_tree_from_transactions;
use crate::core::miner::Miner;
use crate::core::transaction::{TransactionInput, TransactionOutput};
use crate::core::{
    merkle_tree, Address, Block, BlockTree, BlockValidator, ChainContext, Coolcoin, OrphanedBlocks,
    Sha256, Transaction, TransactionPool, UtxoContext, UtxoPool,
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
        let genesis_block = Self::genesis_block();
        Self {
            block_tree: BlockTree::new(genesis_block),
            orphaned_blocks: OrphanedBlocks::new(),
        }
    }

    pub fn tip(&self) -> &BlockHash {
        self.block_tree.tip()
    }

    pub fn all_blocks(&self) -> Vec<Block> {
        let mut all_blocks = vec![];
        for block in &self.block_tree.topological_sort() {
            all_blocks.push(block.clone());
        }

        for block in &self.orphaned_blocks.all() {
            all_blocks.push(block.clone());
        }
        all_blocks
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

    /// Useful for client-side reconstruction of the blockchain.
    pub fn new_block_reinsert_orphans(&mut self, block: Block) {
        if !self.exists(&block) {
            let orphans = self.new_block(block);
            for orphan in orphans {
                self.new_block_reinsert_orphans(orphan);
            }
        }
    }

    pub fn exists(&self, block: &Block) -> bool {
        self.orphaned_blocks.exists(block) || self.block_tree.exists(&block.header().hash())
    }
    pub fn genesis_block() -> Block {
        // 02 Sep 2021 at ~08:58
        let timestamp = 1630569467;
        const GENESIS_REWARD: Coolcoin = Coolcoin::new(50);
        let genesis_address = Address::new("genesis_wallet_address".to_string());
        let locktime = 0;
        let inputs = vec![TransactionInput::new_coinbase()];
        let outputs = vec![TransactionOutput::new(genesis_address, GENESIS_REWARD)];
        let transactions = vec![Transaction::new(inputs, outputs, locktime).unwrap()];
        let previous_block_hash = BlockHash::new(Sha256::new([0; 32]));
        let merkle_root = merkle_tree_from_transactions(&transactions);
        let difficulty = 8;
        let nonce = Miner::pow(&previous_block_hash, &merkle_root, timestamp, difficulty)
            .expect("can't find nonce for genesis block");

        let header = BlockHeader::new(
            previous_block_hash,
            merkle_root,
            timestamp,
            difficulty,
            nonce,
        );
        Block::new(header, transactions)
    }
}
