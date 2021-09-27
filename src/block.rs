use crate::{MerkleHash, MerkleTree, Sha256, Transaction};
use std::fmt::{Display, Formatter};
use std::hash::Hash;

/// A block hash that identifies the block uniquely and unambiguously, and implicitly all of its
/// ancestors.
#[derive(Hash, Ord, PartialOrd, Eq, PartialEq, Debug, Copy, Clone)]
pub struct BlockHash(Sha256);

impl BlockHash {
    pub fn new(hash: Sha256) -> Self {
        Self(hash)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0.as_slice()
    }

    pub fn as_sha256(&self) -> &Sha256 {
        &self.0
    }
}

impl Display for BlockHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Displays the block hash as a hex-encoded string.
        write!(f, "{}", &self.0)
    }
}

/// Block header represents the metadata of the block associated with it.
#[derive(Debug, Clone)]
pub struct BlockHeader {
    // Version number ignored.
    // A reference to the hash of the previous (parent) block in the chain.
    previous_block_hash: BlockHash,
    // A hash of the root of the Merkle tree of this block's transactions.
    merkle_root: MerkleHash,
    // The approximate creation time of this block (seconds from Unix Epoch).
    // LearnCoin timestamp runs out 2106 because it's represented with 32-bits.
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
        // In reality, we should serialize the block header:
        //   - 4 bytes for version
        //   - 32 bytes for previous block hash
        //   - 32 bytes for merkle root
        //   - 4 bytes for timestamp
        //   - 4 bytes for difficulty target
        //   - 4 bytes for nonce
        // All fields should be serialized using the little-endian format.
        // This would ensure that the hash is computed based on values that are both
        // language- and platform- independent.
        // However, we are not going to do this because it doesn't affect our goals, which is
        // to learn the core concepts of the blockchain.
        // This applies to all other hashes in this project.
        // If there is a demand, we are going to do this properly in the future.
        let data = format!(
            "{}{}{}{}{}",
            self.previous_block_hash,
            self.merkle_root,
            self.timestamp,
            self.difficulty_target,
            self.nonce
        );
        // Hash the block header twice.
        let first_hash = Sha256::digest(data.as_bytes());
        let second_hash = Sha256::digest(first_hash.as_slice());
        BlockHash::new(second_hash)
    }

    pub fn previous_block_hash(&self) -> BlockHash {
        self.previous_block_hash
    }

    pub fn merkle_root(&self) -> MerkleHash {
        self.merkle_root
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

#[derive(Debug, Clone)]
pub struct Block {
    // Block hash that is equivalent to `header.hash()`.
    // It's convenient to store it here, rather than having to get it via block header each time.
    id: BlockHash,
    header: BlockHeader,
    // A list of transactions included in this block.
    transactions: Vec<Transaction>,
}

impl Block {
    pub fn new(
        previous_block_hash: BlockHash,
        timestamp: u32,
        difficulty_target: u32,
        nonce: u32,
        transactions: Vec<Transaction>,
    ) -> Self {
        let merkle_root = MerkleTree::merkle_root_from_transactions(&transactions);
        let header = BlockHeader::new(
            previous_block_hash,
            merkle_root,
            timestamp,
            difficulty_target,
            nonce,
        );
        Self {
            id: header.hash(),
            header,
            transactions,
        }
    }

    pub fn id(&self) -> &BlockHash {
        &self.id
    }

    pub fn header(&self) -> &BlockHeader {
        &self.header
    }

    pub fn transactions(&self) -> &Vec<Transaction> {
        &self.transactions
    }
}
