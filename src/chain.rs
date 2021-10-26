use crate::Block;

pub struct Blockchain {
    hashes: Vec<Block>,
}

impl Blockchain {
    pub fn new(genesis_block: Block) -> Self {
        Self {
            hashes: vec![genesis_block],
        }
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
