use crate::{BlockHash, BlockHeader, MerkleHash, Sha256};
use std::cmp::Ordering;

pub struct ProofOfWork {}

impl ProofOfWork {
    /// Returns the nonce such that the corresponding block hash meets the difficulty requirements,
    /// i.e. the block hash is less than or equal to the target hash.
    /// The function returns None if such nonce doesn't exist.
    ///
    /// The target hash is calculated such that all values starting with `difficulty` number of
    /// zeros satisfy the difficulty requirements.
    /// For example, if the difficulty is 5, the numbers (in binary format) starting with 5 zeros
    /// satisfy the criteria.
    pub fn compute_nonce(
        previous_block_hash: &BlockHash,
        merkle_root: &MerkleHash,
        timestamp: u32,
        difficulty: u32,
    ) -> Option<u32> {
        let target_hash = Self::target_hash(difficulty);
        let mut nonce = 0 as u32;
        loop {
            let block_header = BlockHeader::new(
                previous_block_hash.clone(),
                merkle_root.clone(),
                timestamp,
                difficulty,
                nonce,
            );
            if Self::check_difficulty_criteria(&block_header, &target_hash) {
                return Some(nonce);
            }

            if nonce == u32::MAX {
                // We have run out of nonce values, stop the computation.
                break;
            }
            nonce += 1;
        }
        None
    }

    /// Checks whether the given block header is less than or equal to the given target hash.
    fn check_difficulty_criteria(block_header: &BlockHeader, target_hash: &BlockHash) -> bool {
        match block_header.hash().cmp(target_hash) {
            Ordering::Less | Ordering::Equal => true,
            Ordering::Greater => false,
        }
    }

    /// In practice, the target hash is calculated in a more complex way:
    /// https://en.bitcoin.it/wiki/Difficulty
    /// However, for learning purposes, we are going to implement a simpler version which
    /// returns a hash with the first `difficulty` bits set to 0, and the rest set to 1.
    fn target_hash(n_leading_zero_bits: u32) -> BlockHash {
        let mut hash = [0xff; 32];

        // Each byte has 8 bits, so we count how many chunks of 8 bits should be set to 0.
        let num_zero_bytes = (n_leading_zero_bits / 8) as usize;
        for i in 0..num_zero_bytes {
            hash[i] = 0;
        }

        // Represents the number of least significant bits that are ones in the next byte.
        let n_trailing_one_bits = 8 - (n_leading_zero_bits % 8);

        // The special case is required to properly handle overflows, even though mathematically
        // the below algorithm works.
        // For example, 8 ones is 256, and the byte (u8) represents the values from: [0..255].
        if n_trailing_one_bits == 8 {
            return BlockHash::new(Sha256::from_raw(hash));
        }

        // Let's assume that `n_trailing_one_bits` is 5. We want to set the next byte to `00011111`.
        // 2^n_trailing_one_bits is: `00100000`, i.e. `b00100000 - b1 = b00011111`.
        hash[num_zero_bytes] = (1 << n_trailing_one_bits) - 1;
        BlockHash::new(Sha256::from_raw(hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MerkleTree, Transaction, TransactionInput, TransactionOutput};

    #[test]
    fn pow_difficulty_at_least_1_leading_bit_zero() {
        let block_hash = pow_for_difficulty(1);
        assert_eq!(
            block_hash,
            "07e895b142974478ea240cd48f355765834bce88d8a6d7a7bd69fe1d235cb12f"
        )
    }
    #[test]
    fn pow_difficulty_at_least_1_leading_hex_zero() {
        let block_hash = pow_for_difficulty(4);
        assert_eq!(
            block_hash,
            "017dccd5ce1b753350ae5748e499724724b17c4f338e4f3a7c28f136b9710449"
        )
    }

    #[test]
    fn pow_difficulty_at_least_2_leading_hex_zeros() {
        let block_hash = pow_for_difficulty(8);
        assert_eq!(
            block_hash,
            "008fca9bbc13bffa701a4d710f432d43a84386f39918c2654266f4f8ac3fbb98"
        )
    }

    #[test]
    fn pow_difficulty_at_least_4_leading_hex_zeros() {
        let block_hash = pow_for_difficulty(16);
        assert_eq!(
            block_hash,
            "0000dbd87137b69f940833df4e9f9f028f4da4a02fafe16cf4a4bf03a9c1e4e4"
        )
    }

    #[test]
    fn pow_difficulty_at_least_6_leading_hex_zeros() {
        let block_hash = pow_for_difficulty(24);
        assert_eq!(
            block_hash,
            "000000c8d72860b39e3bb975e7f0e11c454580bdba417cb7cf4eea60708bf23e"
        )
    }

    #[test]
    fn probability_test() {
        const DIFFICULTY: u32 = 7;
        const NUM_MINED_BLOCKS: u64 = 500_000;
        let expected_probability: f64 = 1.0 / (2.0 as f64).powf(DIFFICULTY as f64);

        let previous_block_hash = BlockHash::new(Sha256::from_raw([0; 32]));
        let merkle_root = MerkleTree::merkle_root_from_transactions(&create_transactions());

        let mut total_nonces = 0 as u64;
        // We are using a timestamp to modify the block header, and ensure its block hash is
        // different from block hashes of other blocks in this test.
        for timestamp in 0..(NUM_MINED_BLOCKS as u32) {
            let nonce = ProofOfWork::compute_nonce(
                &previous_block_hash,
                &merkle_root,
                timestamp,
                DIFFICULTY,
            )
            .unwrap();
            total_nonces += nonce as u64;
        }

        let actual_probability = (NUM_MINED_BLOCKS as f64) / (total_nonces as f64);
        // Assert that the relative error is less than 1%.
        assert!((expected_probability - actual_probability) / expected_probability < 0.01);
    }

    fn pow_for_difficulty(difficulty: u32) -> String {
        let parent_hash = BlockHash::new(Sha256::from_raw([0; 32]));
        let merkle_root = MerkleTree::merkle_root_from_transactions(&create_transactions());
        let timestamp = 123456;
        let pow_nonce =
            ProofOfWork::compute_nonce(&parent_hash, &merkle_root, timestamp, difficulty).unwrap();
        let block_header = BlockHeader::new(
            parent_hash.clone(),
            merkle_root.clone(),
            timestamp,
            difficulty,
            pow_nonce,
        );
        block_header.hash().as_sha256().to_hex()
    }

    fn create_transactions() -> Vec<Transaction> {
        let address = "example address".to_string();
        let amount = 50;
        let inputs = vec![TransactionInput::new_coinbase()];
        let outputs = vec![TransactionOutput::new(address, amount)];
        vec![Transaction::new(inputs, outputs).unwrap()]
    }
}
