use crate::{
    Block, BlockHash, BlockHeader, BlockTemplate, JsonRpcMethod, JsonRpcRequest, JsonRpcResponse,
    JsonRpcResult, MerkleHash, MerkleTree, PeerConnection, PeerMessagePayload, ProofOfWork,
    PublicKeyAddress, Transaction, TransactionInput, TransactionOutput, VersionMessage,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const INITIAL_BLOCK_REWARD: i64 = 50;
const MINER_VERSION: u32 = 1;
const NONCE_BATCH_SIZE: u32 = 1_000_000;
const NUM_BLOCKS_AFTER_REWARD_IS_HALVED: u32 = 2016;

pub struct MinerParams {
    // Address at which TCP server runs (listens for peer connections).
    pub server_address: String,
    // Size of the receive buffer for each peer connection.
    pub recv_buffer_size: usize,
}

struct ActiveBlockTemplate {
    block_template: BlockTemplate,
    previous_block_hash: BlockHash,
    difficulty_target: u32,
    height: u32,
    public_key_address: PublicKeyAddress,
    current_time: u64,
    merkle_root: MerkleHash,
    transactions: Vec<Transaction>,
}

pub struct Miner {
    connection: PeerConnection,
    next_json_rpc_request_id: u64,
    active_block_template: Option<ActiveBlockTemplate>,
    in_flight_get_block_template: Option<u64>,
    is_handshake_complete: bool,
    checkpoint_nonce: u32,
}

impl Miner {
    pub fn new(params: MinerParams) -> Result<Self, String> {
        let connection = PeerConnection::connect(params.server_address, params.recv_buffer_size)?;
        Ok(Self {
            connection,
            next_json_rpc_request_id: 0,
            active_block_template: None,
            in_flight_get_block_template: None,
            is_handshake_complete: false,
            checkpoint_nonce: 0,
        })
    }

    pub fn run(mut self) -> Result<(), String> {
        self.connection
            .send(&PeerMessagePayload::Version(VersionMessage::new(
                MINER_VERSION,
            )))?;
        loop {
            for message in self.connection.receive_all()? {
                self.on_message(message);
            }

            if !self.is_handshake_complete {
                continue;
            }

            if self.in_flight_get_block_template.is_none() {
                self.send_get_block_template()?;
            }

            match &self.active_block_template {
                None => {
                    // No block template available, keep trying.
                    std::thread::sleep(Duration::from_millis(1000));
                }
                Some(active_block_template) => {
                    // Ensure the stop nonce doesn't overflow.
                    let start_nonce = self.checkpoint_nonce;
                    let stop_nonce = if u32::MAX - NONCE_BATCH_SIZE < start_nonce {
                        u32::MAX
                    } else {
                        start_nonce + NONCE_BATCH_SIZE
                    };
                    println!(
                        "Active template, mining. Start: {}. Stop: {}",
                        start_nonce, stop_nonce
                    );
                    let nonce = ProofOfWork::compute_nonce_with_checkpoint(
                        &active_block_template.previous_block_hash,
                        &active_block_template.merkle_root,
                        active_block_template.current_time,
                        active_block_template.difficulty_target,
                        start_nonce,
                        stop_nonce,
                    );

                    match nonce {
                        None => {
                            // No valid nonce has been found.
                            if stop_nonce == u32::MAX {
                                // The miner has exhausted all possible nonce values.
                                // Drop the current active block.
                                self.clear_active_block_template();
                            } else {
                                self.checkpoint_nonce = stop_nonce + 1;
                            }
                        }
                        Some(valid_nonce) => {
                            self.submit_block(valid_nonce)?;
                            self.clear_active_block_template();
                        }
                    }
                }
            }
        }
    }

    fn on_message(&mut self, message: PeerMessagePayload) {
        match message {
            PeerMessagePayload::Version(_) => {
                // The miner sends the version message, so it doesn't expect this from the server.
                eprintln!("Unexpected message from the server: {:#?}", message);
            }
            PeerMessagePayload::Verack => {
                self.is_handshake_complete = true;
            }
            PeerMessagePayload::JsonRpcRequest(request) => {
                // Miner doesn't respond to JSON RPC requests.
                eprintln!(
                    "Unexpected JSON RPC request from the server: {:#?}",
                    request
                );
            }
            PeerMessagePayload::JsonRpcResponse(response) => self.on_json_rpc_response(response),
            _ => {
                // Miner understands only JSON RPC protocol.
                eprintln!("Unexpected message from the server: {:#?}", message);
            }
        }
    }

    fn on_json_rpc_response(&mut self, response: JsonRpcResponse) {
        match &response.result {
            Ok(JsonRpcResult::Notification) => {
                // Submit request has succeeded.
            }
            Ok(JsonRpcResult::BlockTemplate(block_template)) => {
                self.on_block_template(response.id, block_template);
            }
            Err(e) => {
                // None of the requests should fail.
                eprintln!("Unexpected failed result for: {:#?}: {}", response, e);
            }
            Ok(unexpected) => {
                eprintln!("Unexpected result: {:#?}", unexpected);
            }
        }
    }

    fn on_block_template(&mut self, id: u64, block_template: &BlockTemplate) {
        assert_eq!(self.in_flight_get_block_template, Some(id));
        match &self.active_block_template {
            None => self.update_active_block_template(block_template),
            Some(active_block_template) => {
                if active_block_template.block_template != *block_template {
                    self.update_active_block_template(block_template);
                }
            }
        }
        self.in_flight_get_block_template = None;
    }

    fn update_active_block_template(&mut self, block_template: &BlockTemplate) {
        let mut transactions = vec![Self::make_coinbase_transaction(
            Self::calculate_block_reward(block_template.height),
            &block_template.public_key_address,
        )];
        transactions.extend_from_slice(block_template.transactions.as_slice());
        let merkle_root = MerkleTree::merkle_root_from_transactions(&transactions);
        self.checkpoint_nonce = 0;
        self.active_block_template = Some(ActiveBlockTemplate {
            block_template: block_template.clone(),
            previous_block_hash: block_template.previous_block_hash,
            difficulty_target: block_template.difficulty_target,
            height: block_template.height,
            public_key_address: block_template.public_key_address.clone(),
            current_time: block_template.current_time,
            merkle_root,
            transactions,
        });
    }

    fn submit_block(&mut self, valid_nonce: u32) -> Result<(), String> {
        let id = self.generate_next_json_rpc_request_id();
        // Safety: Active block template always exists when calling submit_block function.
        let active_block_template = self.active_block_template.as_ref().unwrap();
        let block = Block::new(
            active_block_template.previous_block_hash,
            active_block_template.current_time,
            active_block_template.difficulty_target,
            valid_nonce,
            active_block_template.transactions.clone(),
        );
        let method = JsonRpcMethod::SubmitBlock(block);
        let json_rpc_request = JsonRpcRequest { id, method };
        self.connection
            .send(&PeerMessagePayload::JsonRpcRequest(json_rpc_request))?;
        Ok(())
    }

    fn send_get_block_template(&mut self) -> Result<(), String> {
        assert!(self.in_flight_get_block_template.is_none());
        let id = self.generate_next_json_rpc_request_id();
        let method = JsonRpcMethod::GetBlockTemplate;
        let json_rpc_request = JsonRpcRequest { id, method };
        self.connection
            .send(&PeerMessagePayload::JsonRpcRequest(json_rpc_request))?;
        self.in_flight_get_block_template = Some(id);
        Ok(())
    }

    fn generate_next_json_rpc_request_id(&mut self) -> u64 {
        let id = self.next_json_rpc_request_id;
        self.next_json_rpc_request_id += 1;
        id
    }

    fn clear_active_block_template(&mut self) {
        self.active_block_template = None;
        self.checkpoint_nonce = 0;
    }

    fn make_coinbase_transaction(
        block_reward: i64,
        _public_key_address: &PublicKeyAddress,
    ) -> Transaction {
        // TODO: Use public key address to create the unlocking script.
        let inputs = vec![TransactionInput::new_coinbase()];
        let outputs = vec![TransactionOutput::new(block_reward)];
        // Safety: The constructed transaction is always valid.
        Transaction::new(inputs, outputs).unwrap()
    }

    fn calculate_block_reward(height: u32) -> i64 {
        INITIAL_BLOCK_REWARD >> (height / NUM_BLOCKS_AFTER_REWARD_IS_HALVED)
    }
}
