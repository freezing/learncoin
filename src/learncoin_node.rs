use crate::{
    Block, BlockHash, BlockHeader, Blockchain, GetBlocks, Inventory, LearnCoinNetwork, MerkleTree,
    NetworkParams, PeerMessagePayload, ProofOfWork, Sha256, Transaction, TransactionInput,
    TransactionOutput, VersionMessage,
};
use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::Duration;

const MAX_INVENTORY_SIZE: usize = 500;
const MAX_LOCATOR_SIZE: usize = 100;

pub struct LearnCoinNode {
    network: LearnCoinNetwork,
    version: u32,
    blockchain: Blockchain,
    // A list of peers for which the local node expects a verack message.
    verack_message_peers: HashSet<String>,
    // A list of peers for which the local node expects a version message.
    version_message_peers: HashSet<String>,
    // A collection of peers that may have higher active blockchain.
    // This is maintained by processing inventory messages.
    peers_that_are_maybe_ahead: HashSet<String>,
    // Tracks which peers haven't responded back to the getblocks message.
    in_flight_get_blocks: HashSet<String>,
    // Contains block hashes that are not known to the local node, but are known to a peer.
    blocks_advertised_by_peers: HashMap<String, Vec<BlockHash>>,
}

impl LearnCoinNode {
    pub fn connect(network_params: NetworkParams, version: u32) -> Result<Self, String> {
        let network = LearnCoinNetwork::connect(network_params)?;
        Ok(Self {
            network,
            version,
            blockchain: Blockchain::new(Self::genesis_block()),
            verack_message_peers: HashSet::new(),
            version_message_peers: HashSet::new(),
            peers_that_are_maybe_ahead: HashSet::new(),
            in_flight_get_blocks: HashSet::new(),
            blocks_advertised_by_peers: HashMap::new(),
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
        println!(
            "Genesis: {:#?}",
            self.blockchain.block_tree().active_blockchain()
        );
        // A peer that initiates a connection must send the version message.
        // We broadcast the version message to all of our peers before doing any work.
        self.network
            .send_to_all(&PeerMessagePayload::Version(VersionMessage::new(
                self.version,
            )));

        // We expect a verack message from the peers if their version is compatible.
        self.verack_message_peers
            .extend(self.network.peer_addresses().iter().map(|s| s.to_string()));

        loop {
            let new_peers = self.network.accept_new_peers()?;
            if !new_peers.is_empty() {
                println!("New peers connected: {:#?}", new_peers);
            }
            // The local node expects the peers that initiated a connection to send the version
            // messages.
            self.version_message_peers.extend(new_peers);

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
                self.send_messages(&peer_address);
            }

            self.network.drop_inactive_peers();

            // Waiting strategy to avoid busy loops.
            thread::sleep(Duration::from_millis(1));
        }
    }

    fn on_message(&mut self, peer_address: &str, message: PeerMessagePayload) {
        match message {
            PeerMessagePayload::Version(version) => self.on_version(peer_address, version),
            PeerMessagePayload::Verack => self.on_version_ack(peer_address),
            PeerMessagePayload::GetBlocks(get_blocks) => {
                self.on_getblocks(peer_address, get_blocks)
            }
            PeerMessagePayload::Inv(inventory) => self.on_inventory(peer_address, inventory),
        }
    }

    fn on_version(&mut self, peer_address: &str, peer_version: VersionMessage) {
        // We don't expect the version message from this peer anymore.
        let is_version_expected = self.version_message_peers.remove(peer_address);
        if !is_version_expected {
            println!(
                "Received redundant version message from the peer: {}",
                peer_address
            );
            return;
        }

        let is_compatible = peer_version.version() == self.version;
        if !is_compatible {
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
        // Send getblocks message to ensure the local node has caught up with the peer.
        self.network.send(
            peer_address,
            &PeerMessagePayload::GetBlocks(GetBlocks::new(
                self.blockchain.block_tree().locator_hashes(),
            )),
        );
    }

    fn on_version_ack(&mut self, peer_address: &str) {
        let is_removed = self.verack_message_peers.remove(peer_address);
        if !is_removed {
            println!(
                "Received redundant verack message from the peer: {}",
                peer_address
            );
            return;
        }
        // Send getblocks message to ensure the local node has caught up with the peer.
        self.network.send(
            peer_address,
            &PeerMessagePayload::GetBlocks(GetBlocks::new(
                self.blockchain.block_tree().locator_hashes(),
            )),
        );
    }

    fn on_getblocks(&mut self, peer_address: &str, get_blocks: GetBlocks) {
        self.in_flight_get_blocks.remove(peer_address);

        if get_blocks.block_locator_hashes().len() > MAX_LOCATOR_SIZE {
            // The peer doesn't respect the protocol.
            self.close_peer_connection(
                peer_address,
                &format!(
                    "Locator object has {} elements, but maximum allowed is {}",
                    get_blocks.block_locator_hashes().len(),
                    MAX_LOCATOR_SIZE
                ),
            );
            return;
        }

        // Find the first block hash in the locator object that is in the active blockchain.
        // If no such block hash exists, ignore the request.
        let active_blockchain: Vec<BlockHash> = self
            .blockchain
            .block_tree()
            .active_blockchain()
            .iter()
            .map(|b| b.header().hash())
            .collect();
        for locator_hash in get_blocks.block_locator_hashes() {
            match active_blockchain
                .iter()
                .position(|block_hash| block_hash == locator_hash)
            {
                None => {
                    // No such block exists in the active blockchain.
                }
                Some(locator_hash_index) => {
                    // Found a block that exists in the active block chain.
                    // Respond with the inventory containing up to next MAX_INVENTORY_SIZE
                    // block hashes.
                    let hashes = active_blockchain
                        .into_iter()
                        .skip(locator_hash_index + 1)
                        .take(MAX_INVENTORY_SIZE)
                        .collect();
                    self.network.send(
                        peer_address,
                        &PeerMessagePayload::Inv(Inventory::new(hashes)),
                    );
                    return;
                }
            }
        }

        // The sender doesn't respect the protocol because each locator object should include
        // the genesis block.
        self.close_peer_connection(
            peer_address,
            &format!("Locator object must include the genesis block."),
        );
    }

    fn on_inventory(&mut self, peer_address: &str, inventory: Inventory) {
        // We assume that the peer is not ahead anymore.
        self.peers_that_are_maybe_ahead.remove(peer_address);

        for block_hash in inventory.hashes() {
            if !self.blockchain.exists(block_hash) {
                // Our assumption is wrong, and the peer is still ahead.
                self.peers_that_are_maybe_ahead
                    .insert(peer_address.to_string());
                self.blocks_advertised_by_peers
                    .entry(peer_address.to_string())
                    .or_insert_with(|| vec![])
                    .push(block_hash.clone());
            }
        }
    }

    fn send_messages(&mut self, peer_address: &str) {
        // Check if we need to send the getblocks message.
        // We send getblocks message if we caught up with the peer's inventory, and the peer may
        // still be ahead.
        let has_caught_up_with_peer = self
            .blocks_advertised_by_peers
            .get(peer_address)
            .map(Vec::is_empty)
            .unwrap_or(true);
        let is_peer_ahead = self.peers_that_are_maybe_ahead.contains(peer_address);
        let is_get_blocks_in_flight = self.in_flight_get_blocks.contains(peer_address);
        if has_caught_up_with_peer && is_peer_ahead && !is_get_blocks_in_flight {
            let block_locator_hashes = self.blockchain.block_tree().locator_hashes();
            self.network.send(
                peer_address,
                &PeerMessagePayload::GetBlocks(GetBlocks::new(block_locator_hashes)),
            );
            self.in_flight_get_blocks.insert(peer_address.to_string());
        }
    }

    fn close_peer_connection(&mut self, peer_address: &str, reason: &str) {
        self.network.close_peer_connection(peer_address);
        // Free any resources allocated for the peer.
        self.verack_message_peers.remove(peer_address);
        self.version_message_peers.remove(peer_address);
        self.peers_that_are_maybe_ahead.remove(peer_address);
        self.in_flight_get_blocks.remove(peer_address);
        self.blocks_advertised_by_peers.remove(peer_address);
        eprintln!(
            "Closed a connection to the peer {}. Reason: {}",
            peer_address, reason
        );
    }
}
