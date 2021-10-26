use crate::block_index::BlockIndex;
use crate::{
    ActiveChain, Block, BlockHash, BlockHeader, BlockLocatorObject, LearnCoinNetwork, MerkleTree,
    NetworkParams, PeerMessagePayload, PeerState, ProofOfWork, Sha256, Transaction,
    TransactionInput, TransactionOutput, VersionMessage,
};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

const MAX_HEADERS_SIZE: u32 = 2000;

pub struct LearnCoinNode {
    network: LearnCoinNetwork,
    version: u32,
    peer_states: HashMap<String, PeerState>,
    block_index: BlockIndex,
    active_chain: ActiveChain,
    sync_node: Option<String>,
    is_initial_header_sync_complete: bool,
}

impl LearnCoinNode {
    pub fn connect(network_params: NetworkParams, version: u32) -> Result<Self, String> {
        let mut peer_states = HashMap::new();
        for peer_address in network_params.peers() {
            peer_states.insert(peer_address.to_string(), PeerState::new());
        }
        // Initial headers sync is automatically complete if this node is the only node in
        // the network.
        let is_initial_header_sync_complete = network_params.peers().is_empty();
        let network = LearnCoinNetwork::connect(network_params)?;
        let active_chain = ActiveChain::new(Self::genesis_block());

        Ok(Self {
            network,
            version,
            peer_states,
            block_index: BlockIndex::new(Self::genesis_block()),
            active_chain,
            sync_node: None,
            is_initial_header_sync_complete,
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
            let new_peers = self.network.accept_new_peers()?;
            if !new_peers.is_empty() {
                println!("New peers connected: {:#?}", new_peers);
            }
            // The local node expects the peers that initiated a connection to send the version
            // messages.
            for peer_address in &new_peers {
                let mut peer_state = PeerState::new();
                peer_state.expect_version_message = true;
                self.peer_states
                    .insert(peer_address.to_string(), peer_state);
            }

            // Receive data from the network.
            let all_messages = self.network.receive_all();
            for (peer_address, messages) in all_messages {
                for message in messages {
                    self.on_message(&peer_address, message);
                }
            }

            for peer_address in self.peer_addresses() {
                self.maybe_send_messages(&peer_address);
            }

            self.network.drop_misbehaving_peers();

            // Waiting strategy to avoid busy loops.
            thread::sleep(Duration::from_millis(1));
        }
    }

    fn maybe_send_messages(&mut self, peer_address: &str) {
        let peer_state = self.peer_states.get_mut(peer_address).unwrap();

        // Send initial HEADERS message.
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
        }
    }

    fn on_message(&mut self, peer_address: &str, message: PeerMessagePayload) {
        match message {
            PeerMessagePayload::Version(version) => self.on_version(peer_address, version),
            PeerMessagePayload::Verack => self.on_version_ack(peer_address),
            PeerMessagePayload::GetHeaders(block_locator_object) => {
                self.on_get_headers(peer_address, block_locator_object)
            }
            PeerMessagePayload::Headers(headers) => self.on_headers(peer_address, headers),
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

    fn on_headers(&mut self, peer_address: &str, headers: Vec<BlockHeader>) {
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
                        // If the empty headers comes from the sync node, then the node has caught
                        // up with the sync node's view of the active chain.
                        // Therefore, the initial header sync is complete.
                        self.is_initial_header_sync_complete = true;
                        self.sync_node = None;
                    }
                }
            }
            Some(last_new_block_hash) => {
                // Sync node has sent us new information. Request more headers.
                let locator = self.block_index.locator(&last_new_block_hash);
                self.network
                    .send(peer_address, &PeerMessagePayload::GetHeaders(locator));
            }
        }
    }

    fn close_peer_connection(&mut self, peer_address: &str, reason: &str) {
        self.network.close_peer_connection(peer_address);
        // Free any resources allocated for the peer.
        self.peer_states.remove(peer_address);
        eprintln!(
            "Closed a connection to the peer {}. Reason: {}",
            peer_address, reason
        );
    }

    fn peer_addresses(&self) -> Vec<String> {
        self.network
            .peer_addresses()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }
}
