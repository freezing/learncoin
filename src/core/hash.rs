use crate::core::block::BlockHash;
use crate::core::Transaction;
use serde::de::{EnumAccess, Error, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::Digest;
use std::fmt::{Display, Formatter};

#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct Sha256([u8; 32]);

impl Sha256 {
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    pub fn bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Serialize for Sha256 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(as_hex(self.bytes()).as_str())
    }
}

struct StringVisitor;

impl<'de> Visitor<'de> for StringVisitor {
    type Value = Sha256;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match hex::decode(&v) {
            Ok(bytes) => {
                let mut sha = [0; 32];
                if bytes.len() == 32 {
                    for i in 0..32 {
                        sha[i] = *bytes.get(i).unwrap();
                    }
                    Ok(Sha256::new(sha))
                } else {
                    Err(E::custom(format!(
                        "Invalid sha length. Expected: {} but got: {} in: {}",
                        32,
                        bytes.len(),
                        v
                    )))
                }
            }
            Err(e) => Err(E::custom(e.to_string())),
        }
    }
}

impl<'de> Deserialize<'de> for Sha256 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(StringVisitor)
    }
}

impl Display for Sha256 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", as_hex(&self.bytes()[..]))
    }
}

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
        write!(f, "{}", hex::encode(self.0.bytes()))
    }
}

pub fn as_hex(bytes: &[u8]) -> String {
    hex::encode(bytes)
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
    Sha256::new(output)
}

/// In practice, the target hash is calculated in a more complex way:
/// https://en.bitcoin.it/wiki/Difficulty
/// However, for learning purposes, we are going to implement a simpler version which
/// returns a hash with bit 1 set at index that equals difficulty - 1.
/// I.e. this means that difficulty represents how many leading zeroes the block hash must have.
pub fn target_hash(n_zero_bits: u32) -> BlockHash {
    let mut hash = [0xff; 32];

    let num_zero_bytes = (n_zero_bits / 8) as usize;
    for i in 0..num_zero_bytes {
        hash[i] = 0;
    }

    let remainder = 8 - (n_zero_bits % 8);
    if remainder == 8 {
        return BlockHash::new(Sha256::new(hash));
    }

    hash[num_zero_bytes] = (1 << remainder) - 1;
    BlockHash::new(Sha256::new(hash))
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
            let mut concat = lhs.bytes().iter().map(|x| *x).collect::<Vec<u8>>();
            concat.extend_from_slice(rhs.bytes());
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

    #[test]
    fn target_hash_test() {
        assert_eq!(
            as_hex(target_hash(0).raw()),
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
        );
        assert_eq!(
            as_hex(target_hash(4).raw()),
            "0fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
        );
        assert_eq!(
            as_hex(target_hash(8).raw()),
            "00ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
        );
        assert_eq!(
            as_hex(target_hash(12).raw()),
            "000fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
        );
        assert_eq!(
            as_hex(target_hash(16).raw()),
            "0000ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
        );
        assert_eq!(
            as_hex(target_hash(20).raw()),
            "00000fffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
        );
    }
}
