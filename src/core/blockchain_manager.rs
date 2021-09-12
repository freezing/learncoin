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
        for block in &self.block_tree.all() {
            all_blocks.push(block.clone());
        }

        for block in &self.orphaned_blocks.all() {
            all_blocks.push(block.clone());
        }
        all_blocks
    }

    pub fn orphaned_blocks(&self) -> Vec<Block> {
        self.orphaned_blocks.all()
    }

    pub fn block_tree(&self) -> &BlockTree {
        &self.block_tree
    }

    /// Assumes that the block is valid.
    pub fn new_block(&mut self, block: Block) -> Vec<Block> {
        if self.block_tree.exists(block.header().previous_block_hash()) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::hash::{from_hex, MerkleHash};

    #[test]
    fn new_block_reinsert_orphans() {
        const DIFFICULTY_TARGET: u32 = 1;

        let mut blockchain = BlockchainManager::new();
        let block_0 = BlockchainManager::genesis_block();
        let block_1 = Block::new(
            BlockHeader::new(
                block_0.id().clone(),
                MerkleHash::new(
                    from_hex("00cf8be900cf8be900cf8be900cf8be900cf8be900cf8be900cf8be900cf8be9")
                        .unwrap(),
                ),
                100,
                DIFFICULTY_TARGET,
                3,
            ),
            vec![],
        );
        let block_2 = Block::new(
            BlockHeader::new(
                block_1.id().clone(),
                MerkleHash::new(
                    from_hex("0005e6c10005e6c10005e6c10005e6c10005e6c10005e6c10005e6c10005e6c1")
                        .unwrap(),
                ),
                100,
                DIFFICULTY_TARGET,
                3,
            ),
            vec![],
        );
        let block_3 = Block::new(
            BlockHeader::new(
                block_2.id().clone(),
                MerkleHash::new(
                    from_hex("00d8368100d8368100d8368100d8368100d8368100d8368100d8368100d83681")
                        .unwrap(),
                ),
                100,
                DIFFICULTY_TARGET,
                3,
            ),
            vec![],
        );

        blockchain.new_block_reinsert_orphans(block_2.clone());
        blockchain.new_block_reinsert_orphans(block_3.clone());

        {
            // Assert block_2 and block_3 are orphans, and only genesis block is in the active blockchain.
            {
                // Orphans check.
                let mut actual = blockchain
                    .orphaned_blocks
                    .all()
                    .iter()
                    .map(|b| b.id().clone())
                    .collect::<Vec<BlockHash>>();
                actual.sort();
                let expected = vec![block_3.id().clone(), block_2.id().clone()];
                assert_eq!(actual, expected);
            }

            {
                // Active blockchain check.
                let actual = blockchain
                    .block_tree()
                    .active_blockchain()
                    .iter()
                    .map(|b| b.id().clone())
                    .collect::<Vec<BlockHash>>();
                assert_eq!(actual, vec![block_0.id().clone()]);
            }
        }

        {
            blockchain.new_block_reinsert_orphans(block_1.clone());
            // Assert that inserting block_1 inserts blocks 2 and 3.
            // This leaves us with no orphans, and active blockchain should contain all nodes.
            {
                // Orphans check.
                let mut actual = blockchain
                    .orphaned_blocks
                    .all()
                    .iter()
                    .map(|b| b.id().clone())
                    .collect::<Vec<BlockHash>>();
                actual.sort();
                assert_eq!(actual, vec![]);
            }

            {
                // Active blockchain check.
                let actual = blockchain
                    .block_tree()
                    .active_blockchain()
                    .iter()
                    .map(|b| b.id().clone())
                    .collect::<Vec<BlockHash>>();
                assert_eq!(
                    actual,
                    vec![
                        block_0.id().clone(),
                        block_1.id().clone(),
                        block_2.id().clone(),
                        block_3.id().clone()
                    ]
                );
            }
        }
    }
}
