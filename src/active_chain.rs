use crate::Block;

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

    pub fn genesis(&self) -> &Block {
        self.hashes.first().unwrap()
    }

    pub fn tip(&self) -> &Block {
        self.hashes.last().unwrap()
    }

    pub fn accept_block(&mut self, block: Block) {
        self.hashes.push(block);
    }

    pub fn hashes(&self) -> &Vec<Block> {
        &self.hashes
    }
}