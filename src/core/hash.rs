use crate::core::block::BlockHash;
use serde::{Deserialize, Serialize};

pub type Sha256 = [u8; 64];

/// In practice, the target hash is calculated in a more complex way:
/// https://en.bitcoin.it/wiki/Difficulty
/// However, for learning purposes, we are going to implement a simpler version which
/// returns a hash with bit 1 set at index that equals difficulty - 1.
/// I.e. this means that difficulty represents how many leading zeroes the block hash must have.
pub fn target_hash(difficulty: u32) -> BlockHash {
    let mut hash = [0; 64];
    hash[(difficulty - 1) as usize] = 1;
    BlockHash::new(hash)
}
