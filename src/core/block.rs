use crate::core::{Sha256, Transaction};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

// TODO: Move to merkle_tree module later.
#[derive(Debug)]
pub struct MerkleHash(Vec<u8>);

#[derive(Hash, Ord, PartialOrd, Eq, PartialEq, Debug, Copy, Clone)]
pub struct BlockHash(Sha256);

impl BlockHash {
    pub fn new(hash: Sha256) -> Self {
        Self(hash)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl Display for BlockHash {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!("Write as hex value")
    }
}

#[derive(Debug)]
pub struct BlockHeader {
    // Version number ignored.
    // A reference to the hash of the previous (parent) block in the chain.
    previous_block_hash: BlockHash,
    // A hash of the root of the merkle tree of this block's transactions.
    merkle_root: MerkleHash,
    // The approximate creation time of this block (seconds from Unix Epoch).
    timestamp: u32,
    // The Proof-of-Work algorithm difficulty target for this block.
    difficulty_target: u32,
    // A counter used for the Proof-of-Work algorithm.
    nonce: u32,
}

impl BlockHeader {
    pub fn new(
        previous_block_hash: BlockHash,
        merkle_root: MerkleHash,
        timestamp: u32,
        difficulty_target: u32,
        nonce: u32,
    ) -> Self {
        Self {
            previous_block_hash,
            merkle_root,
            timestamp,
            difficulty_target,
            nonce,
        }
    }

    pub fn hash(&self) -> BlockHash {
        todo!("Get SHA-256 of the header")
    }
    pub fn timestamp(&self) -> u32 {
        self.timestamp
    }
    pub fn difficulty_target(&self) -> u32 {
        self.difficulty_target
    }
    pub fn nonce(&self) -> u32 {
        self.nonce
    }
    pub fn previous_block_hash(&self) -> &BlockHash {
        &self.previous_block_hash
    }
    pub fn merkle_root(&self) -> &MerkleHash {
        &self.merkle_root
    }
}

#[derive(Debug)]
pub struct Block {
    header: BlockHeader,
    transactions: Vec<Transaction>,
}

impl Block {
    pub fn new(header: BlockHeader, transactions: Vec<Transaction>) -> Self {
        Self {
            header,
            transactions,
        }
    }

    pub fn header(&self) -> &BlockHeader {
        &self.header
    }

    pub fn transactions(&self) -> &Vec<Transaction> {
        &self.transactions
    }
}
