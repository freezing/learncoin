use crate::core::Transaction;
use std::hash::Hash;

pub struct BlockHeader {
    // Version number ignored.
    // A reference to the hash of the previous (parent) block in the chain.
    previous_block_hash: Vec<u8>,
    // A hash of the root of the merkle tree of this block's transactions.
    merkle_root: Vec<u8>,
    // The approximate creation time of this block (seconds from Unix Epoch).
    timestamp: u32,
    // The Proof-of-Work algorithm difficulty target for this block.
    difficulty_target: u32,
    // A counter used for the Proof-of-Work algorithm.
    nonce: u32,
}

impl BlockHeader {
    pub fn new(
        previous_block_hash: Vec<u8>,
        merkle_root: Vec<u8>,
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

    pub fn previous_block_hash(&self) -> &Vec<u8> {
        &self.previous_block_hash
    }
    pub fn merkle_root(&self) -> &Vec<u8> {
        &self.merkle_root
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
}

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
