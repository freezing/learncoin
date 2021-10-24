use crate::BlockHash;

pub struct BlockLocatorObject {
    hashes: Vec<BlockHash>,
}

impl BlockLocatorObject {
    pub fn new(hashes: Vec<BlockHash>) -> Self {
        BlockLocatorObject { hashes }
    }

    pub fn hashes(&self) -> &Vec<BlockHash> {
        &self.hashes
    }
}
