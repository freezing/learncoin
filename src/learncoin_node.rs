use crate::{
    Block, BlockHash, Blockchain, LearnCoinNetwork, MerkleTree, NetworkParams, PeerMessagePayload,
    ProofOfWork, Sha256, Transaction, TransactionInput, TransactionOutput, VersionMessage,
};
use std::collections::HashSet;
use std::thread;
use std::time::Duration;

pub struct LearnCoinNode {
    network: LearnCoinNetwork,
    version: u32,
    blockchain: Blockchain,
    // A list of peers from which the local node expects a verack message.
    peers_to_receive_verack_from: HashSet<String>,
    // A list of peers from which the local node expects a version message.
    peers_to_receive_version_from: HashSet<String>,
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

            self.network.drop_inactive_peers();

            // Waiting strategy to avoid busy loops.
            thread::sleep(Duration::from_millis(1));
        }
    }

    fn on_message(&mut self, peer_address: &str, message: PeerMessagePayload) {
        match message {
            PeerMessagePayload::Version(version) => self.on_version(peer_address, version),
            PeerMessagePayload::Verack => self.on_version_ack(peer_address),
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
