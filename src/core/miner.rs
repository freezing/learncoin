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
