use crate::block_index::BlockIndex;
use crate::block_storage::BlockStorage;
use crate::{
    ActiveChain, Block, BlockHash, BlockHeader, BlockLocatorObject, JsonRpcMethod, JsonRpcRequest,
    JsonRpcResponse, LearnCoinNetwork, MerkleTree, NetworkParams, PeerMessagePayload, PeerState,
    ProofOfWork, Sha256, Transaction, TransactionInput, TransactionOutput, VersionMessage,
};
use std::cmp::min;
use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

const MAX_HEADERS_SIZE: u32 = 2000;
const BLOCK_DOWNLOAD_WINDOW: usize = 1024;
const MAX_BLOCKS_IN_TRANSIT_PER_PEER: usize = 16;

const HEADERS_RESPONSE_TIMEOUT: Duration = Duration::from_millis(60_000);
const GET_BLOCK_DATA_RESPONSE_TIMEOUT: Duration = Duration::from_millis(60_000);

struct InFlightBlockRequest {
    peer_address: String,
    sent_at: Instant,
}

pub struct LearnCoinNode {
    network: LearnCoinNetwork,
    version: u32,
    peer_states: HashMap<String, PeerState>,
    block_index: BlockIndex,
    block_storage: BlockStorage,
    active_chain: ActiveChain,
    sync_node: Option<String>,
    is_initial_header_sync_complete: bool,
    in_flight_block_requests: HashMap<BlockHash, InFlightBlockRequest>,
}

impl LearnCoinNode {
    pub fn connect(network_params: NetworkParams, version: u32) -> Result<Self, String> {
        let mut peer_states = HashMap::new();
        let genesis_block = Self::genesis_block();
        for peer_address in network_params.peers() {
            peer_states.insert(
                peer_address.to_string(),
                PeerState::new(*genesis_block.id()),
            );
        }
        // Initial headers sync is automatically complete if this node is the only node in
        // the network.
        let is_initial_header_sync_complete = network_params.peers().is_empty();
        let network = LearnCoinNetwork::connect(network_params)?;
        let active_chain = ActiveChain::new(genesis_block.clone());

        Ok(Self {
            network,
            version,
            peer_states,
            block_index: BlockIndex::new(genesis_block.clone()),
            block_storage: BlockStorage::new(genesis_block),
            active_chain,
            sync_node: None,
            is_initial_header_sync_complete,
            in_flight_block_requests: HashMap::new(),
        })
    }

    pub fn genesis_block() -> Block {
        // 02 Sep 2021 at ~08:58
        let timestamp = 1630569467;
        const GENESIS_REWARD: i64 = 50;
        let inputs = vec![TransactionInput::new_coinbase()];
        let outputs = vec![TransactionOutput::new(GENESIS_REWARD)];
        let transactions = vec![Transaction::new(inputs, outputs).unwrap()];
        let previous_block_hash = BlockHash::new(Sha256::from_raw([0; 32]));
        let merkle_root = MerkleTree::merkle_root_from_transactions(&transactions);
        // An arbitrary initial difficulty.
        let difficulty = 8;
        let nonce =
            ProofOfWork::compute_nonce(&previous_block_hash, &merkle_root, timestamp, difficulty)
                .expect("can't find nonce for the genesis block");
        Block::new(
            previous_block_hash,
            timestamp,
            difficulty,
            nonce,
            transactions,
        )
    }

    pub fn run(mut self) -> Result<(), String> {
        // A peer that initiates a connection must send the version message.
        // We send the version message to all of our peers before doing any work.
        for peer_address in self.peer_addresses() {
            self.network.send(
                &peer_address,
                &PeerMessagePayload::Version(VersionMessage::new(self.version)),
            );
            self.peer_states
                .get_mut(&peer_address)
                .unwrap()
                .expect_verack_message = true;
        }

        loop {
            let current_time = Instant::now();

            let new_peers = self.network.accept_new_peers()?;
            if !new_peers.is_empty() {
                println!("New peers connected: {:#?}", new_peers);
            }
            // The local node expects the peers that initiated a connection to send the version
            // messages.
            for peer_address in &new_peers {
                let mut peer_state = PeerState::new(*self.active_chain.genesis().id());
                peer_state.expect_version_message = true;
                self.peer_states
                    .insert(peer_address.to_string(), peer_state);
            }

            // Receive data from the network.
            let all_messages = self.network.receive_all();
            for (peer_address, messages) in all_messages {
                for message in messages {
                    self.on_message(&peer_address, message, current_time);
                }
            }

            for peer_address in self.peer_addresses() {
                self.maybe_send_messages(&peer_address, current_time);
                self.check_timeouts(&peer_address, current_time);
            }

            self.network.drop_misbehaving_peers();

            // Waiting strategy to avoid busy loops.
            thread::sleep(Duration::from_millis(1));
        }
    }

    fn maybe_send_messages(&mut self, peer_address: &str, current_time: Instant) {
        self.maybe_send_initial_headers(peer_address, current_time);
        self.maybe_send_get_block_data(peer_address, current_time);
    }

    fn maybe_send_initial_headers(&mut self, peer_address: &str, current_time: Instant) {
        let peer_state = self.peer_states.get_mut(peer_address).unwrap();

        if self.is_initial_header_sync_complete {
            return;
        }

        let is_handshake_complete =
            !peer_state.expect_verack_message && !peer_state.expect_version_message;
        if !is_handshake_complete {
            return;
        }

        if self.sync_node.is_none() {
            self.sync_node = Some(peer_address.to_string());
            let locator = self.block_index.locator(self.active_chain.tip().id());
            self.network
                .send(peer_address, &PeerMessagePayload::GetHeaders(locator));
            peer_state.headers_message_sent_at = Some(current_time);
        }
    }

    fn maybe_send_get_block_data(&mut self, peer_address: &str, current_time: Instant) {
        let Self {
            is_initial_header_sync_complete,
            peer_states,
            block_index,
            block_storage,
            in_flight_block_requests,
            ..
        } = self;

        if !*is_initial_header_sync_complete {
            return;
        }
        let peer_state = peer_states.get_mut(peer_address).unwrap();

        assert!(MAX_BLOCKS_IN_TRANSIT_PER_PEER >= peer_state.num_blocks_in_transit);
        let num_free_slots = MAX_BLOCKS_IN_TRANSIT_PER_PEER - peer_state.num_blocks_in_transit;
        let blocks_to_download = Self::find_next_blocks_to_download(
            peer_state,
            block_index,
            block_storage,
            in_flight_block_requests,
            num_free_slots,
        );
        if !blocks_to_download.is_empty() {
            peer_state.num_blocks_in_transit += blocks_to_download.len();
            for block_to_download in &blocks_to_download {
                let previous = in_flight_block_requests.insert(
                    *block_to_download,
                    InFlightBlockRequest {
                        peer_address: peer_address.to_string(),
                        sent_at: current_time,
                    },
                );
                assert!(previous.is_none());
            }
            self.network.send(
                peer_address,
                &PeerMessagePayload::GetBlockData(blocks_to_download),
            );
        }
    }

    fn find_next_blocks_to_download(
        peer_state: &mut PeerState,
        block_index: &mut BlockIndex,
        block_storage: &mut BlockStorage,
        in_flight_block_requests: &mut HashMap<BlockHash, InFlightBlockRequest>,
        num_blocks_to_download: usize,
    ) -> Vec<BlockHash> {
        let last_common_block_index_node = block_index
            .get_block_index_node(&peer_state.last_common_block)
            .unwrap();
        let last_known_block_index_node = block_index
            .get_block_index_node(&peer_state.last_known_hash)
            .unwrap();
        let window_end = last_common_block_index_node.height + BLOCK_DOWNLOAD_WINDOW;
        let max_height = min(window_end, last_known_block_index_node.height);

        let mut blocks_to_download = vec![];
        let mut index_walk_node = last_common_block_index_node;
        while index_walk_node.height < max_height
            && num_blocks_to_download > blocks_to_download.len()
        {
            let num_remaining_slots = num_blocks_to_download - blocks_to_download.len();
            let num_blocks_to_fetch = min(max_height - index_walk_node.height, num_remaining_slots);
            index_walk_node = block_index
                .ancestor(
                    &last_known_block_index_node.block_header.hash(),
                    index_walk_node.height + num_blocks_to_fetch,
                )
                .unwrap();
            let mut candidate_blocks = vec![index_walk_node.block_header.hash()];
            for i in 0..(num_blocks_to_fetch - 1) {
                let last = candidate_blocks.last().unwrap();
                assert_ne!(*last, peer_state.last_common_block);
                let parent = block_index.parent(last).unwrap();
                candidate_blocks.push(parent);
            }
            // Candidate blocks are inserted in reverse, i.e. children before parents.
            // Order them such that parents come first.
            candidate_blocks.reverse();

            for candidate in &candidate_blocks {
                let is_already_downloaded = block_storage.exists(candidate);
                let is_in_flight = in_flight_block_requests.contains_key(&candidate);

                if !is_already_downloaded && !is_in_flight {
                    blocks_to_download.push(*candidate);
                }
            }

            for candidate in &candidate_blocks {
                let is_already_downloaded = block_storage.exists(candidate);
                if !is_already_downloaded {
                    break;
                }
                // For the time being, we assume that the blocks are valid, so we update the last common block
                // if the block is fully downloaded.
                // However, this will change.
                peer_state.last_common_block = *candidate;
            }
        }
        blocks_to_download
    }

    fn check_timeouts(&mut self, peer_address: &str, current_time: Instant) {
        self.check_timeouts_get_headers(peer_address, current_time);
        self.check_timeouts_get_block_data(peer_address, current_time);
    }

    fn check_timeouts_get_headers(&mut self, peer_address: &str, current_time: Instant) {
        let peer_state = self.peer_states.get_mut(peer_address).unwrap();
        // get headers
        match peer_state.headers_message_sent_at {
            None => {
                // Nothing to do since there are no in-flight headers messages.
            }
            Some(headers_message_sent_at) => {
                let elapsed = current_time.duration_since(headers_message_sent_at);
                if elapsed.gt(&HEADERS_RESPONSE_TIMEOUT) {
                    self.close_peer_connection(
                        peer_address,
                        &format!(
                            "Response to getheaders has timed out after {} ms",
                            HEADERS_RESPONSE_TIMEOUT.as_millis()
                        ),
                    );

                    // If it's the sync node that didn't respond, it is not a sync node anymore.
                    if let Some(sync_node) = &self.sync_node {
                        if sync_node == peer_address {
                            self.sync_node = None;
                        }
                    }
                }
            }
        }
    }

    fn check_timeouts_get_block_data(&mut self, peer_address: &str, current_time: Instant) {
        let mut expired_block_requests = vec![];
        for (block_hash, in_flight_block_request) in &self.in_flight_block_requests {
            let elapsed = current_time.duration_since(in_flight_block_request.sent_at);
            if elapsed.gt(&GET_BLOCK_DATA_RESPONSE_TIMEOUT) {
                expired_block_requests.push(*block_hash);
            }
        }

        for block_hash in expired_block_requests {
            let expired_request = self.in_flight_block_requests.remove(&block_hash).unwrap();
            self.close_peer_connection(
                &expired_request.peer_address,
                &format!(
                    "Response to get block data has timed out after {} ms for block {}.",
                    GET_BLOCK_DATA_RESPONSE_TIMEOUT.as_millis(),
                    block_hash
                ),
            );
        }
    }

    fn on_message(
        &mut self,
        peer_address: &str,
        message: PeerMessagePayload,
        current_time: Instant,
    ) {
        match message {
            PeerMessagePayload::Version(version) => self.on_version(peer_address, version),
            PeerMessagePayload::Verack => self.on_version_ack(peer_address),
            PeerMessagePayload::GetHeaders(block_locator_object) => {
                self.on_get_headers(peer_address, block_locator_object)
            }
            PeerMessagePayload::Headers(headers) => {
                self.on_headers(peer_address, headers, current_time)
            }
            PeerMessagePayload::GetBlockData(block_hashes) => {
                self.on_get_block_data(peer_address, block_hashes)
            }
            PeerMessagePayload::Block(block) => self.on_block(peer_address, block),
            PeerMessagePayload::JsonRpcRequest(request) => {
                self.on_json_rpc_request(peer_address, request)
            }
            PeerMessagePayload::JsonRpcResponse(response) => {
                self.on_json_rpc_response(peer_address, response)
            }
        }
    }

    fn on_version(&mut self, peer_address: &str, peer_version: VersionMessage) {
        let peer_state = self.peer_states.get_mut(peer_address).unwrap();

        if !peer_state.expect_version_message {
            println!(
                "Received redundant version message from the peer: {}",
                peer_address
            );
            return;
        }
        peer_state.expect_version_message = false;

        let is_version_compatible = peer_version.version() == self.version;
        if !is_version_compatible {
            self.close_peer_connection(
                peer_address,
                &format!(
                    "Version is not compatible. Expected {} but peer's version is: {}",
                    self.version,
                    peer_version.version()
                ),
            );
            return;
        }

        // The version is compatible, send the verack message to the peer.
        self.network.send(peer_address, &PeerMessagePayload::Verack);
    }

    fn on_version_ack(&mut self, peer_address: &str) {
        let peer_state = self.peer_states.get_mut(peer_address).unwrap();
        if !peer_state.expect_verack_message {
            println!(
                "Received redundant verack message from the peer: {}",
                peer_address
            );
            return;
        }
        peer_state.expect_verack_message = false;
    }

    fn on_get_headers(&mut self, peer_address: &str, block_locator_object: BlockLocatorObject) {
        let active_blockchain = self
            .active_chain
            .hashes()
            .iter()
            .map(|block| block.header().clone())
            .collect::<Vec<BlockHeader>>();

        // Find the first block hash in the locator object that is in the active blockchain.
        // If no such block hash exists, the peer is misbehaving because at least the genesis block
        // should exist.
        for locator_hash in block_locator_object.hashes() {
            match active_blockchain
                .iter()
                .position(|header| header.hash() == *locator_hash)
            {
                None => {
                    // No such block exists in the active blockchain.
                }
                Some(locator_hash_index) => {
                    // Found a block that exists in the active blockchain.
                    // Respond with the headers containing up to MAX_HEADERS_SIZE block hashes.
                    let headers = active_blockchain
                        .into_iter()
                        .skip(locator_hash_index + 1)
                        .take(MAX_HEADERS_SIZE as usize)
                        .collect();
                    self.network
                        .send(peer_address, &PeerMessagePayload::Headers(headers));
                    return;
                }
            }
        }

        // No blocks from the locator object have been found. The peer is misbehaving.
        self.close_peer_connection(
            peer_address,
            &format!("Locator object must include the genesis block"),
        );
    }

    fn on_headers(&mut self, peer_address: &str, headers: Vec<BlockHeader>, current_time: Instant) {
        let mut peer_state = self.peer_states.get_mut(peer_address).unwrap();
        peer_state.headers_message_sent_at = None;

        let mut last_new_block_hash = None;
        for header in headers {
            if !self.block_index.exists(&header.previous_block_hash()) {
                // Peer is misbehaving, the headers don't connect.
                self.close_peer_connection(
                    peer_address,
                    "Peer is misbehaving. The block headers do not connect.",
                );
                return;
            } else if !self.block_index.exists(&header.hash()) {
                last_new_block_hash = Some(header.hash());
                self.block_index.insert(header);
            }
        }

        match last_new_block_hash {
            None => {
                // Do not request any more headers from the peer,
                // because it doesn't have anything new.
                if let Some(sync_node) = &self.sync_node {
                    if sync_node == peer_address {
                        let sync_node_state = self.peer_states.get(sync_node).unwrap();

                        // If the empty headers comes from the sync node, then the node has caught
                        // up with the sync node's view of the active chain.
                        // Therefore, the initial header sync is complete.
                        self.is_initial_header_sync_complete = true;
                        self.sync_node = None;

                        println!("Initial headers sync complete.");

                        // Send headers message to each peer.
                        // It's okay to send a redundant headers message to the sync node,
                        // which will respond with another empty headeres message.
                        for peer_address in self.peer_addresses() {
                            // Use last known hash from the sync node as that's the latest
                            // block hash the node knows about.
                            let locator =
                                self.block_index.locator(&sync_node_state.last_known_hash);
                            self.network
                                .send(&peer_address, &PeerMessagePayload::GetHeaders(locator));
                        }
                    }
                }
            }
            Some(last_new_block_hash) => {
                // Sync node has sent us new information. Request more headers.
                let locator = self.block_index.locator(&last_new_block_hash);
                self.network
                    .send(peer_address, &PeerMessagePayload::GetHeaders(locator));
                peer_state.headers_message_sent_at = Some(current_time);
                peer_state.last_known_hash = last_new_block_hash;
            }
        }
    }

    fn on_get_block_data(&mut self, peer_address: &str, block_hashes: Vec<BlockHash>) {
        for block_hash in block_hashes {
            match self.block_storage.get(&block_hash) {
                None => {
                    // Peer has requested invalid block.
                    self.close_peer_connection(
                        peer_address,
                        &format!(
                            "Peer is misbehaving. Requested invalid block: {}",
                            block_hash
                        ),
                    );
                    return;
                }
                Some(block) => {
                    self.network
                        .send(peer_address, &PeerMessagePayload::Block(block.clone()));
                }
            }
        }
    }

    fn on_block(&mut self, peer_address: &str, block: Block) {
        let peer_state = self.peer_states.get_mut(peer_address).unwrap();
        peer_state.num_blocks_in_transit -= 1;
        self.in_flight_block_requests.remove(block.id());
        self.block_storage.insert(block.clone());
    }

    fn on_json_rpc_request(&mut self, peer_address: &str, request: JsonRpcRequest) {
        let JsonRpcRequest { id, method } = request;
        match method {
            JsonRpcMethod::Placeholder => {}
        }
    }

    fn on_json_rpc_response(&mut self, peer_address: &str, response: JsonRpcResponse) {
        // Node doesn't use JSON-RPC to invoke methods on any node, so it doesn't expect any
        // responses in return.
        // The peer is misbehaving.
        self.close_peer_connection(
            peer_address,
            &format!("Received JSON-RPC response: {:#?}", response),
        );
    }

    fn close_peer_connection(&mut self, peer_address: &str, reason: &str) {
        self.network.close_peer_connection(peer_address);
        // Free any resources allocated for the peer.
        let existing = self.peer_states.remove(peer_address);
        if existing.is_some() {
            eprintln!(
                "Closed a connection to the peer {}. Reason: {}",
                peer_address, reason
            );
        }
    }

    fn peer_addresses(&self) -> Vec<String> {
        self.network
            .peer_addresses()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }
}
