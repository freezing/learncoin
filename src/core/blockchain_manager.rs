use crate::core::block::{BlockHash, BlockHeader};
use crate::core::miner::Miner;
use crate::core::transaction::{TransactionInput, TransactionOutput};
use crate::core::{
    merkle_tree, Address, Block, BlockTree, BlockValidator, ChainContext, Coolcoin, OrphanedBlocks,
    Transaction, TransactionPool, UtxoContext, UtxoPool,
};

/// Responsible for processing new blocks and new transactions from the network.
/// It validates that blocks and transactions are valid.
/// TODO: Maybe can be called Blockchain?
pub struct BlockchainManager {
    block_tree: BlockTree,
    orphaned_blocks: OrphanedBlocks,
}

impl BlockchainManager {
    pub fn new(current_time: u32) -> Self {
        let genesis_block = Self::genesis_block(current_time);
        Self {
            block_tree: BlockTree::new(genesis_block),
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
    fn genesis_block(current_time: u32) -> Block {
        const GENESIS_REWARD: Coolcoin = Coolcoin::new(50);
        // TODO: Generate genesis address.
        let genesis_address = Address::new([0; 32]);
        let locktime = 0;
        let inputs = vec![TransactionInput::new_coinbase()];
        let outputs = vec![TransactionOutput::new(genesis_address, GENESIS_REWARD)];
        let transactions = vec![Transaction::new(inputs, outputs, locktime).unwrap()];
        let previous_block_hash = BlockHash::new([0; 32]);
        let leaves = transactions
            .iter()
            .map(|tx| &tx.id().raw()[..])
            .collect::<Vec<&[u8]>>();
        let merkle_root = merkle_tree(&leaves);
        let difficulty = 1;
        let nonce = Miner::pow(&previous_block_hash, &merkle_root, current_time, difficulty)
            .expect("can't find nonce for genesis block");

        let header = BlockHeader::new(
            previous_block_hash,
            merkle_root,
            current_time,
            difficulty,
            nonce,
        );
        Block::new(header, transactions)
    }
}
