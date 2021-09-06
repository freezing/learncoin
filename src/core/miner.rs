use std::cmp::Ordering;
use std::sync::mpsc;
use std::sync::mpsc::{
    Receiver, RecvTimeoutError, SendError, Sender, SyncSender, TryRecvError, TrySendError,
};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::core::block::{BlockHash, BlockHeader};
use crate::core::hash::{merkle_tree_from_transactions, MerkleHash};
use crate::core::{merkle_tree, target_hash, Block, Sha256, Transaction};

#[derive(Debug)]
pub struct MinerRequest {
    previous_block_hash: BlockHash,
    transactions: Vec<Transaction>,
    difficulty_target: u32,
}

impl MinerRequest {
    pub fn new(
        previous_block_hash: BlockHash,
        transactions: Vec<Transaction>,
        difficulty_target: u32,
    ) -> Self {
        Self {
            previous_block_hash,
            transactions,
            difficulty_target,
        }
    }
}

#[derive(Debug)]
pub enum MinerResponse {
    None(MinerRequest),
    Mined(Block),
}

pub struct Miner {
    rx: Receiver<MinerRequest>,
    tx: Sender<MinerResponse>,
}

pub struct MinerChannel {
    miner_requests: Sender<MinerRequest>,
    miner_responses: Receiver<MinerResponse>,
    num_outstanding_requests: u32,
}

impl MinerChannel {
    pub fn send(&mut self, request: MinerRequest) -> Result<(), String> {
        let result = self.miner_requests.send(request).map_err(|e| e.to_string());
        if result.is_ok() {
            self.num_outstanding_requests += 1;
        }
        result
    }

    pub fn read(&mut self) -> Result<MinerResponse, TryRecvError> {
        let result = self.miner_responses.try_recv();
        if result.is_ok() {
            self.num_outstanding_requests -= 1;
        }
        result
    }

    pub fn num_outstanding_requests(&self) -> u32 {
        self.num_outstanding_requests
    }
}

impl Miner {
    pub fn start_async() -> MinerChannel {
        const TIMEOUT: Duration = Duration::from_secs(1);
        let (miner_requests, rx) = mpsc::channel();
        let (tx, miner_responses) = mpsc::channel();

        thread::spawn(move || loop {
            // todo!("Flush all, keep only the last request.");
            match rx.recv_timeout(TIMEOUT) {
                Ok(request) => {
                    println!("Miner received a new request: {:#?}", request);
                    let MinerRequest {
                        previous_block_hash,
                        transactions,
                        difficulty_target,
                    } = request;

                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as u32;
                    let merkle_root = merkle_tree_from_transactions(&transactions);
                    let block_nonce = Self::pow(
                        &previous_block_hash,
                        &merkle_root,
                        timestamp,
                        difficulty_target,
                    );
                    let response = match block_nonce {
                        None => MinerResponse::None(MinerRequest {
                            previous_block_hash,
                            transactions,
                            difficulty_target,
                        }),
                        Some(nonce) => {
                            let header = BlockHeader::new(
                                previous_block_hash,
                                merkle_root,
                                timestamp,
                                difficulty_target,
                                nonce,
                            );
                            MinerResponse::Mined(Block::new(header, transactions))
                        }
                    };
                    tx.send(response).unwrap();
                }
                Err(_e) => {
                    // eprintln!("{}", _e.to_string());
                    continue;
                }
            }
        });

        MinerChannel {
            miner_requests,
            miner_responses,
            num_outstanding_requests: 0,
        }
    }

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
    use crate::core::{as_hex, BlockchainManager};

    use super::*;

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
