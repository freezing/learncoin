use crate::core::peer_connection::PeerMessage;
use crate::core::{BlockchainManager, CoolcoinNetwork};
use std::net::TcpStream;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

/// There are four roles in the Coolcoin P2P network:
///   - Wallet: A function of a wallet is to send and receive Coolcoins.
///             It may be part of the full node, which is usually the case with desktop clients.
///   - Miner: A function of the mining node is to produce new blocks with unconfirmed transactions.
///            Some mining nodes are also full nodes.
///   - Full Blockchain: Responsible for validating transactions and blocks.
///                      Full blockchain nodes can autonomously and authoritatively verify
///                      blocks and transactions without external reference.
///   - Network routing node: A function of the routing node is to relay information about blocks
///     and transactions to the blockchain network.
///     All nodes have this role.
///             
/// CoolcoinNode has the following roles:
///   - Miner
///   - Full Blockchain
///   - Network routing node
pub struct CoolcoinNode {
    network: CoolcoinNetwork,
    blockchain_manager: BlockchainManager,
}

impl CoolcoinNode {
    pub fn connect(network: CoolcoinNetwork) -> Result<Self, String> {
        Ok(Self {
            network,
            blockchain_manager: BlockchainManager::new(),
        })
    }

    pub fn run(mut self) {
        loop {
            // Accept new peers.
            match self.network.accept_new_peers() {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("Error while accepting new peers: {}", e);
                }
            }

            // Receive data from the network.
            let messages = self.network.receive_data();
            for (sender, message) in messages {
                match self.on_message(&sender, message) {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("Error while processing new message: {}", e);
                    }
                }
            }
            sleep(Duration::from_millis(100));
        }
    }

    fn on_message(&mut self, sender: &str, message: PeerMessage) -> Result<(), String> {
        match message {
            PeerMessage::GetBlocks(block_height) => self.on_get_blocks(sender, block_height),
        }
    }

    fn on_get_blocks(&mut self, sender: &str, block_height: u32) -> Result<(), String> {
        todo!("Send inventory status to the sender")
    }
}
