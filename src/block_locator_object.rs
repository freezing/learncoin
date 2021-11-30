use crate::BlockHash;
use serde::{Deserialize, Serialize};

/// Block locator object is a list of block hashes that describe what hashes exist in a blockchain.
/// It always includes the 10 last hashes.
/// Each next N block hashes are skipped, where N grows exponentially after 10 hashes.
/// Block hashes are sorted by their height in descending order.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
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
