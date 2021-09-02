use crate::core::block::BlockHash;
use crate::core::Transaction;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::fmt::{Display, Formatter};

pub type Sha256 = [u8; 32];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MerkleHash(Sha256);

impl MerkleHash {
    pub fn new(hash: Sha256) -> MerkleHash {
        Self(hash)
    }

    pub fn raw(&self) -> &Sha256 {
        &self.0
    }
}

impl Display for MerkleHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

pub fn as_hex(sha: &Sha256) -> String {
    hex::encode(sha)
}

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

pub fn merkle_tree(leaves: &Vec<&[u8]>) -> MerkleHash {
    assert!(!leaves.is_empty());
    let mut hashes = leaves
        .iter()
        .map(|leaf| hash(*leaf))
        .collect::<Vec<Sha256>>();

    while hashes.len() != 1 {
        if hashes.len() % 2 == 1 {
            hashes.push(hashes.last().unwrap().clone());
        }

        let mut next_level_hashes = vec![];

        for i in (0..hashes.len()).step_by(2) {
            let lhs = hashes.get(i).unwrap();
            let rhs = hashes.get(i + 1).unwrap();
            let mut concat = lhs.iter().map(|x| *x).collect::<Vec<u8>>();
            concat.extend_from_slice(rhs);
            next_level_hashes.push(hash(&concat))
        }

        hashes = next_level_hashes
    }
    MerkleHash::new(hashes.into_iter().next().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{as_hex, merkle_tree};

    #[test]
    fn hash_test() {
        let data = b"hello world";
        assert_eq!(
            hex::encode(hash(data)),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn merkle_tree_even() {
        let merkle_root = merkle_tree(&vec![b"hello", b"world", b"this is", b"coolcoin"]);
        assert_eq!(
            as_hex(merkle_root.raw()),
            "9a78c5b0f711a613e62660182f4357c7befd179d27c57cf8abb6e31a23d1cd7b"
        );
    }

    #[test]
    fn merkle_tree_odd() {
        let merkle_root = merkle_tree(&vec![b"hello", b"world", b"this is"]);
        assert_eq!(
            as_hex(merkle_root.raw()),
            "be1257a768ca532e01caed9b6cdc420a52f3de14dd5adcb353066cf581334c35"
        );
    }

    #[test]
    fn merkle_tree_even_same_as_previous_odd() {
        let merkle_root = merkle_tree(&vec![b"hello", b"world", b"this is", b"this is"]);
        assert_eq!(
            as_hex(merkle_root.raw()),
            "be1257a768ca532e01caed9b6cdc420a52f3de14dd5adcb353066cf581334c35"
        );
    }
}
