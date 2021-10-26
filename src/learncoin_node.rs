use crate::block_index::BlockIndex;
use crate::{
    Block, BlockHash, BlockHeader, BlockLocatorObject, Blockchain, LearnCoinNetwork, MerkleTree,
    NetworkParams, PeerMessagePayload, ProofOfWork, Sha256, Transaction, TransactionInput,
    TransactionOutput, VersionMessage,
};
use std::collections::HashSet;
use std::hash::Hash;
use std::thread;
use std::time::Duration;

const MAX_HEADERS_SIZE: u32 = 2000;

pub struct LearnCoinNode {
    network: LearnCoinNetwork,
    version: u32,
    block_tree: BlockIndex,
    active_blockchain: Blockchain,
    // A list of peers from which the local node expects a verack message.
    peers_to_receive_verack_from: HashSet<String>,
    // A list of peers from which the local node expects a version message.
    peers_to_receive_version_from: HashSet<String>,
    initial_block_download_in_progress: bool,
}

impl LearnCoinNode {
    pub fn connect(network_params: NetworkParams, version: u32) -> Result<Self, String> {
        let network = LearnCoinNetwork::connect(network_params)?;
        let mut active_blockchain = Blockchain::new(Self::genesis_block());

        Ok(Self {
            network,
            version,
            block_tree: BlockIndex::new(Self::genesis_block()),
            active_blockchain,
            peers_to_receive_verack_from: HashSet::new(),
            peers_to_receive_version_from: HashSet::new(),
            initial_block_download_in_progress: false,
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
        // We broadcast the version message to all of our peers before doing any work.
        self.network
            .send_to_all(&PeerMessagePayload::Version(VersionMessage::new(
                self.version,
            )));

        // We expect a verack message from the peers if their version is compatible.
        self.peers_to_receive_verack_from
            .extend(self.network.peer_addresses().iter().map(|s| s.to_string()));

        loop {
            let new_peers = self.network.accept_new_peers()?;
            if !new_peers.is_empty() {
                println!("New peers connected: {:#?}", new_peers);
            }
            // The local node expects the peers that initiated a connection to send the version
            // messages.
            self.peers_to_receive_version_from.extend(new_peers);

            // Receive data from the network.
            let all_messages = self.network.receive_all();
            for (peer_address, messages) in all_messages {
                for message in messages {
                    self.on_message(&peer_address, message);
                }
            }

            let peer_addresses = self
                .network
                .peer_addresses()
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
            for peer_address in peer_addresses {
                self.maybe_send_messages(&peer_address);
            }

            self.network.drop_inactive_peers();

            // Waiting strategy to avoid busy loops.
            thread::sleep(Duration::from_millis(1));
        }
    }

    fn maybe_send_messages(&mut self, peer_address: &str) {
        let is_handshake_complete = !self.peers_to_receive_verack_from.contains(peer_address)
            && !self.peers_to_receive_version_from.contains(peer_address);
        if !is_handshake_complete {
            return;
        }

        if !self.initial_block_download_in_progress {
            self.initial_block_download_in_progress = true;
            let locator = self.block_tree.locator(self.active_blockchain.tip().id());
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
        // We don't expect the version message from this peer anymore.
        let is_version_expected_from_peer = self.peers_to_receive_version_from.remove(peer_address);
        if !is_version_expected_from_peer {
            println!(
                "Received redundant version message from the peer: {}",
                peer_address
            );
            return;
        }

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
        let is_removed = self.peers_to_receive_verack_from.remove(peer_address);
        if !is_removed {
            println!(
                "Received redundant verack message from the peer: {}",
                peer_address
            );
            return;
        }
    }

    fn on_get_headers(&mut self, peer_address: &str, block_locator_object: BlockLocatorObject) {
        let active_blockchain = self
            .active_blockchain
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
        if headers.is_empty() {
            // Nothing to do here. Also, we do not request any more headers from the peer.
            return;
        }

        // Store headers in the block tree.
        for header in headers {
            if !self.block_tree.exists(&header.previous_block_hash()) {
                // Peer is misbehaving, the headers don't connect.
                self.close_peer_connection(
                    peer_address,
                    "Peer is misbehaving. The block headers do not connect.",
                );
                return;
            }
            self.block_tree.insert(header);
        }

        // We request new headers from the peer until we get an empty headers message.`
        let locator = self.block_tree.locator(self.active_blockchain.tip().id());
        self.network
            .send(peer_address, &PeerMessagePayload::GetHeaders(locator));
    }

    fn close_peer_connection(&mut self, peer_address: &str, reason: &str) {
        self.network.close_peer_connection(peer_address);
        // Free any resources allocated for the peer.
        self.peers_to_receive_verack_from.remove(peer_address);
        self.peers_to_receive_version_from.remove(peer_address);
        eprintln!(
            "Closed a connection to the peer {}. Reason: {}",
            peer_address, reason
        );
    }
}
