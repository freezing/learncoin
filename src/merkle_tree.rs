use crate::{Sha256, Transaction};
use std::fmt::{Display, Formatter};

/// Represents a SHA-256 hash of a Merkle tree node.
#[derive(Debug, Copy, Clone)]
pub struct MerkleHash(Sha256);

impl MerkleHash {
    pub fn new(hash: Sha256) -> MerkleHash {
        Self(hash)
    }

    pub fn raw(&self) -> &Sha256 {
        &self.0
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0.as_slice()
    }
}

/// Contains a logic to construct a Merkle tree.
pub struct MerkleTree;

impl MerkleTree {
    pub fn merkle_root_from_transactions(transactions: &Vec<Transaction>) -> MerkleHash {
        let leaves = transactions
            .iter()
            .map(|tx| tx.id().as_slice())
            .collect::<Vec<&[u8]>>();
        Self::merkle_root(&leaves)
    }

    pub fn merkle_root(leaves: &Vec<&[u8]>) -> MerkleHash {
        assert!(!leaves.is_empty());
        let mut current_level_hashes = leaves
            .iter()
            .map(|leaf| Sha256::digest(*leaf))
            .collect::<Vec<Sha256>>();

        while current_level_hashes.len() != 1 {
            if current_level_hashes.len() % 2 == 1 {
                // If a level has an odd number of nodes, duplicate the last node.
                current_level_hashes.push(current_level_hashes.last().unwrap().clone());
            }

            let mut next_level_hashes = vec![];

            for i in (0..current_level_hashes.len()).step_by(2) {
                let lhs = current_level_hashes.get(i).unwrap();
                let rhs = current_level_hashes.get(i + 1).unwrap();

                // Concatenate children's hashes.
                let mut concat = lhs.as_slice().iter().map(|x| *x).collect::<Vec<u8>>();
                concat.extend_from_slice(rhs.as_slice());

                // The concatenated hash is the value of the new parent node in the next level.
                next_level_hashes.push(Sha256::digest(&concat))
            }

            current_level_hashes = next_level_hashes
        }
        MerkleHash::new(current_level_hashes.into_iter().next().unwrap())
    }
}

impl Display for MerkleHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
