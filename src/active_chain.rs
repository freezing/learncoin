use crate::{Block, BlockHash};

/// Represents an active chain in the blockchain.
pub struct ActiveChain {
    /// A list of blocks starting with the Genesis block
    hashes: Vec<Block>,
}

impl ActiveChain {
    pub fn new(genesis_block: Block) -> Self {
        Self {
            hashes: vec![genesis_block],
        }
    }

    pub fn all_blocks(&self) -> &Vec<Block> {
        &self.hashes
    }

    pub fn genesis(&self) -> &Block {
        self.hashes.first().unwrap()
    }

    pub fn tip(&self) -> &Block {
        self.hashes.last().unwrap()
    }

    pub fn accept_block(&mut self, block: Block) {
        self.hashes.push(block);
    }

    /// Removes the last block in the active chain.
    /// May only be called when the tip is not the genesis block.
    pub fn remove_tip(&mut self) -> Block {
        assert!(self.hashes.len() > 1);
        self.hashes.pop().unwrap()
    }

    pub fn hashes(&self) -> &Vec<Block> {
        &self.hashes
    }
}
