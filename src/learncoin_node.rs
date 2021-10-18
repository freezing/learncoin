use crate::{
    Block, BlockHash, BlockHeader, Blockchain, GetBlocks, Inventory, LearnCoinNetwork, MerkleTree,
    NetworkParams, PeerMessagePayload, ProofOfWork, Sha256, Transaction, TransactionInput,
    TransactionOutput, VersionMessage,
};
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::Duration;

const MAX_INVENTORY_SIZE: usize = 500;
const MAX_LOCATOR_SIZE: usize = 100;
const MAX_IN_FLIGHT_MESSAGES_PER_PEER: u32 = 500;

pub struct LearnCoinNode {
    network: LearnCoinNetwork,
    version: u32,
    blockchain: Blockchain,
    // A list of peers from which the local node expects a verack message.
    peers_to_receive_verack_from: HashSet<String>,
    // A list of peers from which the local node expects a version message.
    peers_to_receive_version_from: HashSet<String>,
    // Blocks that are not in the local node's blockchain and can be retrieved from peers.
    blocks_to_retrieve: HashMap<BlockHash, HashSet<String>>,
    // Peers that are ahead are the ones whose last inventory message wasn't empty.
    peers_that_are_ahead: HashSet<String>,
    // A list of peers from which the local node expects an inventory message.
    peers_to_receive_inventory_from: HashSet<String>,
    // A number of in-flight messages for each peer.
    num_in_flight_messages: HashMap<String, u32>,
}

impl LearnCoinNode {
    pub fn connect(network_params: NetworkParams, version: u32) -> Result<Self, String> {
        let network = LearnCoinNetwork::connect(network_params)?;
        Ok(Self {
            network,
            version,
            blockchain: Blockchain::new(Self::genesis_block()),
            peers_to_receive_verack_from: HashSet::new(),
            peers_to_receive_version_from: HashSet::new(),
            blocks_to_retrieve: HashMap::new(),
            peers_that_are_ahead: HashSet::new(),
            peers_to_receive_inventory_from: HashSet::new(),
            num_in_flight_messages: HashMap::new(),
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
                self.maybe_send_getblocks_message(&peer_address);
            }
            self.maybe_send_getdata_messages();

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
            PeerMessagePayload::GetData(block_hash) => self.on_getdata(peer_address, block_hash),
            PeerMessagePayload::Block(block) => self.on_block(peer_address, block),
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
        // Send getblocks message to ensure the local node has caught up with the peer.
        self.network.send(
            peer_address,
            &PeerMessagePayload::GetBlocks(GetBlocks::new(
                self.blockchain.block_tree().locator_hashes(),
            )),
        );
        self.peers_that_are_ahead.insert(peer_address.to_string());
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
        // Send getblocks message to ensure the local node has caught up with the peer.
        self.network.send(
            peer_address,
            &PeerMessagePayload::GetBlocks(GetBlocks::new(
                self.blockchain.block_tree().locator_hashes(),
            )),
        );
        self.peers_that_are_ahead.insert(peer_address.to_string());
    }

    fn on_getblocks(&mut self, peer_address: &str, get_blocks: GetBlocks) {
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
        self.peers_to_receive_inventory_from.remove(peer_address);
        if inventory.hashes().is_empty() {
            self.peers_that_are_ahead.remove(peer_address);
        }

        for block_hash in inventory.into_hashes() {
            if !self.blockchain.exists(&block_hash) {
                self.blocks_to_retrieve
                    .entry(block_hash)
                    .or_insert_with(|| HashSet::new())
                    .insert(peer_address.to_string());
            }
        }
    }

    fn on_getdata(&mut self, peer_address: &str, block_hash: BlockHash) {
        let block = match self
            .blockchain
            .block_tree()
            .active_blockchain()
            .iter()
            .find(|block| *block.id() == block_hash)
        {
            None => {
                println!("Peer {} requested a block hash {} that doesn't exist in the active blockchain.",
                                 peer_address, block_hash);
                return;
            }
            Some(block) => block.clone(),
        };
        self.network
            .send(peer_address, &PeerMessagePayload::Block(block));
    }

    fn on_block(&mut self, peer_address: &str, block: Block) {
        let mut blocks_to_insert = vec![block];
        while !blocks_to_insert.is_empty() {
            let mut new_blocks_to_insert = vec![];
            for block in blocks_to_insert {
                // TODO: Validate each block before inserting it.
                let orphans = self.blockchain.new_block(block);
                new_blocks_to_insert.extend(orphans);
            }
            blocks_to_insert = new_blocks_to_insert;
        }

        // Each block is a response to getdata message.
        match self.num_in_flight_messages.get_mut(peer_address) {
            None => {
                println!("The peer probably sent us the block without requesting it. Ignore it.")
            }
            Some(mut count) => *count -= 1,
        }
    }

    fn maybe_send_getblocks_message(&mut self, peer_address: &str) {
        // Check if we need to send the getblocks message.
        let is_peer_ahead = self.peers_that_are_ahead.contains(peer_address);
        let has_block_hashes_to_retrieve = !self.blocks_to_retrieve.is_empty();
        let is_getblocks_in_flight = self.peers_to_receive_inventory_from.contains(peer_address);

        if is_peer_ahead && !has_block_hashes_to_retrieve && !is_getblocks_in_flight {
            let block_locator_hashes = self.blockchain.block_tree().locator_hashes();
            self.network.send(
                peer_address,
                &PeerMessagePayload::GetBlocks(GetBlocks::new(block_locator_hashes)),
            );
            self.peers_to_receive_inventory_from
                .insert(peer_address.to_string());
        }
    }

    fn maybe_send_getdata_messages(&mut self) {
        // Required to avoid mutating the blocks_to_retrieve while iterating over it.
        let mut sent_block_hashes = vec![];

        for (block_hash, peers) in &self.blocks_to_retrieve {
            // Convert HashSet to Vector to select a random peer.
            let peers = peers.iter().map(String::to_string).collect::<Vec<String>>();
            let random_index = rand::thread_rng().gen_range(0..(peers.len()));
            let peer = peers.get(random_index).unwrap();

            let mut num_in_flight_messages = self
                .num_in_flight_messages
                .entry(peer.to_string())
                .or_insert(0);
            if *num_in_flight_messages < MAX_IN_FLIGHT_MESSAGES_PER_PEER {
                *num_in_flight_messages += 1;
                self.network
                    .send(peer, &PeerMessagePayload::GetData(block_hash.clone()));
                sent_block_hashes.push(block_hash.clone());
            }
        }

        // Remove the sent block hashes from `block_to_retrieve`.
        for block_hash in sent_block_hashes {
            self.blocks_to_retrieve.remove(&block_hash);
        }
    }

    fn maybe_send_getblock_message(&mut self, peer_address: &str) {}

    fn close_peer_connection(&mut self, peer_address: &str, reason: &str) {
        self.network.close_peer_connection(peer_address);
        // Free any resources allocated for the peer.
        self.peers_to_receive_verack_from.remove(peer_address);
        self.peers_to_receive_version_from.remove(peer_address);
        self.peers_that_are_ahead.remove(peer_address);
        self.peers_to_receive_inventory_from.remove(peer_address);
        eprintln!(
            "Closed a connection to the peer {}. Reason: {}",
            peer_address, reason
        );
    }
}
