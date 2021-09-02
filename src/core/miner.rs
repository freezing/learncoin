use crate::core::block::{BlockHash, BlockHeader};
use crate::core::hash::MerkleHash;
use crate::core::{target_hash, Sha256};
use std::cmp::Ordering;

pub struct Miner {}

impl Miner {
    pub fn pow(
        parent_hash: &BlockHash,
        merkle_root: &MerkleHash,
        timestamp: u32,
        difficulty_target: u32,
    ) -> Option<u32> {
        let target_hash = target_hash(difficulty_target);
        println!("Target hash: {}", target_hash);
        let mut nonce = 0 as u32;
        loop {
            if Self::test_nonce(
                parent_hash,
                merkle_root,
                timestamp,
                difficulty_target,
                nonce,
                &target_hash,
            ) {
                return Some(nonce);
            }

            if nonce == u32::MAX {
                break;
            }
            nonce += 1;
        }
        None
    }

    fn test_nonce(
        parent_hash: &BlockHash,
        merkle_root: &MerkleHash,
        timestamp: u32,
        difficulty_target: u32,
        nonce: u32,
        target_hash: &BlockHash,
    ) -> bool {
        let block = BlockHeader::new(
            parent_hash.clone(),
            merkle_root.clone(),
            timestamp,
            difficulty_target,
            nonce,
        );
        match block.hash().cmp(target_hash) {
            Ordering::Less => true,
            Ordering::Equal | Ordering::Greater => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{as_hex, BlockchainManager};

    #[test]
    fn pow_difficulty_1() {
        let block_hash = pow_difficulty(1);
        assert_eq!(
            block_hash,
            "00b505a7e489ca039fe9197b7e7217e03f4c3003e9418266d3c1eb2f373b276f"
        )
    }
    #[test]
    fn pow_difficulty_4() {
        let block_hash = pow_difficulty(4);
        assert_eq!(
            block_hash,
            "00b505a7e489ca039fe9197b7e7217e03f4c3003e9418266d3c1eb2f373b276f"
        )
    }

    fn pow_difficulty(difficulty: u32) -> String {
        // Use genesis block to avoid manually constructing transactions and other data.
        // Then override data we care about, i.e. difficulty.
        let genesis = BlockchainManager::genesis_block();
        let parent_hash = genesis.header().previous_block_hash();
        let merkle_root = genesis.header().merkle_root();
        let timestamp = genesis.header().timestamp();
        let pow_nonce = Miner::pow(parent_hash, merkle_root, timestamp, difficulty).unwrap();
        let pow_block = BlockHeader::new(
            parent_hash.clone(),
            merkle_root.clone(),
            timestamp,
            difficulty,
            pow_nonce,
        );
        as_hex(pow_block.hash().raw())
    }
}
