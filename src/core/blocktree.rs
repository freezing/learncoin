use crate::core::block::{BlockHash, BlockValidator};
use crate::core::transaction::{TransactionInput, TransactionOutput};
use crate::core::{Address, Block, Coolcoin, Transaction};
use std::collections::hash_map::Entry;
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
    blocks_tree: HashMap<BlockHash, BlockTreeEntry>,
    // A hash of the last block in the active blockchain.
    active_block: ActiveBlock,
}

impl BlockTree {
    // TODO: Take genesis_block as parameter.
    pub fn new() -> Self {
        const GENESIS_DIFFICULTY: u32 = 1;
        let genesis_block = Self::genesis_block();
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            // Bitcoin timestamp runs out in year 2106.
            .as_secs() as u32;
        let target_hash = Self::make_target_hash(GENESIS_DIFFICULTY);
        BlockValidator::validate(&genesis_block, &target_hash, current_time).unwrap();
        let mut blocks = HashMap::new();
        let genesis_hash = genesis_block.header().hash();
        blocks.insert(
            genesis_hash,
            BlockTreeEntry {
                block: genesis_block,
                height: 0,
            },
        );
        Self {
            blocks_tree: blocks,
            active_block: ActiveBlock {
                hash: genesis_hash,
                total_work: 0,
            },
        }
    }

    // TODO: There are three types of validation:
    // - Chain-context validation such as difficulty (needs block height).
    // - Context-free validation, such as block size, coinbase transactions, etc.
    // - UTXO-context validation, such as input values < output values.
    // TODO: Validation should be moved into a Validation concept.
    // It may require additional API to get all the necessary information for the validation.
    //

    /// Adds new block to the blockchain. It assumes that the block is valid and all
    /// necessary validation has been perform before calling this function.
    ///
    /// Preconditions:
    ///   - Parent exists.
    pub fn insert(&mut self, block: Block) {
        let parent_hash = block.header().previous_block_hash();
        let block_hash = block.header().hash();
        let parent = self.blocks_tree.get(parent_hash).unwrap();
        let block_height = parent.height + 1;
        let previous = self.blocks_tree.insert(
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

    /// In practice, the target hash is calculated in a more complex way:
    /// https://en.bitcoin.it/wiki/Difficulty
    /// However, for learning purposes, we are going to implement a simpler version which
    /// returns a hash with bit 1 set at index that equals difficulty - 1.
    /// I.e. this means that difficulty represents how many leading zeroes the block hash must have.
    // TODO: Move somewhere else.
    fn make_target_hash(difficulty: u32) -> BlockHash {
        let mut hash = [0; 64];
        hash[(difficulty - 1) as usize] = 1;
        BlockHash::new(hash)
    }
}
