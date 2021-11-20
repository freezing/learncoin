use crate::{LearnCoinNetwork, NetworkParams, PeerMessagePayload, PeerState, VersionMessage};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

pub struct LearnCoinNode {
    network: LearnCoinNetwork,
    version: u32,
    peer_states: HashMap<String, PeerState>,
}

impl LearnCoinNode {
    pub fn connect(network_params: NetworkParams, version: u32) -> Result<Self, String> {
        let mut peer_states = HashMap::new();
        for peer_address in network_params.peers() {
            peer_states.insert(peer_address.to_string(), PeerState::new());
        }
        let network = LearnCoinNetwork::connect(network_params)?;

        Ok(Self {
            network,
            version,
            peer_states,
        })
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

            self.network.drop_misbehaving_peers();

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
