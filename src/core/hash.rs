use crate::core::block::BlockHash;
use serde::{Deserialize, Serialize};
use sha2::Digest;

pub type Sha256 = [u8; 32];

pub fn hash(data: &[u8]) -> Sha256 {
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    assert_eq!(result.len(), 32);
    let mut output = [0; 32];
    for (i, byte) in result.iter().enumerate() {
        output[i] = *byte;
    }
    output
}

/// In practice, the target hash is calculated in a more complex way:
/// https://en.bitcoin.it/wiki/Difficulty
/// However, for learning purposes, we are going to implement a simpler version which
/// returns a hash with bit 1 set at index that equals difficulty - 1.
/// I.e. this means that difficulty represents how many leading zeroes the block hash must have.
pub fn target_hash(difficulty: u32) -> BlockHash {
    let mut hash = [0; 32];
    hash[(difficulty - 1) as usize] = 1;
    BlockHash::new(hash)
}

#[cfg(test)]
mod tests {
    use crate::core::hash::hash;

    #[test]
    fn hash_test() {
        let data = b"hello world";
        assert_eq!(
            hex::encode(hash(data)),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }
}
