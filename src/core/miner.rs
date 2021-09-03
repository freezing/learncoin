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
            Ordering::Less | Ordering::Equal => true,
            Ordering::Greater => false,
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
    fn pow_difficulty_1_leading_zeroes() {
        let block_hash = pow_difficulty(4);
        assert_eq!(
            block_hash,
            "00a13221f144959b8665fdab0921577255ec34df40869f2139535599094de23a"
        )
    }

    #[test]
    fn pow_difficulty_2_leading_zeroes() {
        let block_hash = pow_difficulty(8);
        assert_eq!(
            block_hash,
            "0000a8bc60c45f850d65260794f72edad849cc878388ba7f8f5cb26ba4bce463"
        )
    }

    #[test]
    fn pow_difficulty_4_leading_zeroes() {
        let block_hash = pow_difficulty(16);
        assert_eq!(
            block_hash,
            "000000746e4dd118ca13ecb03b47f8b35deaa4c5fa933b850e9ff8cf9b785779"
        )
    }

    #[test]
    fn pow_difficulty_7_leading_zeroes() {
        let block_hash = pow_difficulty(28);
        assert_eq!(
            block_hash,
            "008a3fefacbe3cedc3f2d336d2f6d8684f440935888d3f818a1e9edd02619f36"
        )
    }

    #[test]
    fn probability_test() {
        const DIFFICULTY: u32 = 7;
        const BLOCKS_TO_MINE: u64 = 100000;
        const EXPECTED_PER_BLOCK: u64 = 1 << DIFFICULTY;
        const EXPECTED_TOTAL_HASHES: u64 = EXPECTED_PER_BLOCK * BLOCKS_TO_MINE;
        const EXPECTED_TOTAL_HASHES_ERROR: u64 = EXPECTED_TOTAL_HASHES / 20; // Within 5%
        let genesis = BlockchainManager::genesis_block();
        let header = genesis.header();

        let mut total_nonces = 0 as u64;
        for timestamp in 0..(BLOCKS_TO_MINE as u32) {
            let nonce = Miner::pow(
                header.previous_block_hash(),
                header.merkle_root(),
                timestamp,
                DIFFICULTY,
            )
            .unwrap();
            total_nonces += nonce as u64;
        }

        println!("Data: {} {}", EXPECTED_TOTAL_HASHES, total_nonces);
        let diff = if EXPECTED_TOTAL_HASHES >= total_nonces {
            EXPECTED_TOTAL_HASHES - total_nonces
        } else {
            total_nonces - EXPECTED_TOTAL_HASHES
        };
        assert!(diff < EXPECTED_TOTAL_HASHES_ERROR);
    }
    // TODO: Probability test that roughly every 1/D hashes result in a block.

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
