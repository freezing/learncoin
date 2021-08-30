use crate::core::block::BlockHash;
use crate::core::peer_connection::PeerMessage;
use crate::core::{Block, BlockchainManager, CoolcoinNetwork, Transaction};
use std::net::TcpStream;
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
    outstanding_get_inventory_requests: Vec<String>,
}

impl CoolcoinNode {
    pub fn connect(network: CoolcoinNetwork) -> Result<Self, String> {
        Ok(Self {
            network,
            blockchain_manager: BlockchainManager::new(),
            outstanding_get_inventory_requests: Vec::new(),
        })
    }

    pub fn run(mut self) {
        // If we can't send messages to all nodes immediately, then there is no point in trying
        // to recover since this is part of the startup.
        // It is okay for the process to fail since retrying would mean rerunning the process.
        // Of course, in production like implementation we would handle that in code.
        self.network.broadcast(PeerMessage::GetInventory()).unwrap();

        loop {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as u32;

            // Accept new peers.
            match self.network.accept_new_peers() {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("Error while accepting new peers: {}", e);
                }
            }

            // Process outstanding inventory requests.
            let outstanding_requests = self.outstanding_get_inventory_requests.clone();
            self.outstanding_get_inventory_requests.clear();
            for request in outstanding_requests {
                match self.on_get_inventory(&request) {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("Error while processing outstanding requests: {}", e);
                    }
                }
            }

            // Receive data from the network.
            let messages = self.network.receive_all();
            for (sender, message) in messages {
                match self.on_message(&sender, message, current_time) {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("Error while processing new message: {}", e);
                    }
                }
            }
            sleep(Duration::from_millis(100));
        }
    }

    fn on_message(
        &mut self,
        sender: &str,
        message: PeerMessage,
        current_time: u32,
    ) -> Result<(), String> {
        match message {
            PeerMessage::GetInventory() => self.on_get_inventory(sender),
            PeerMessage::ResponseInventory(inventory) => {
                self.on_response_inventory(sender, inventory, current_time)
            }
            PeerMessage::RelayBlock(block) => self.on_relay_block(sender, block, current_time),
            PeerMessage::RelayTransaction(transaction) => {
                self.on_relay_transaction(sender, transaction)
            }
        }
    }

    fn on_get_inventory(&mut self, sender: &str) -> Result<(), String> {
        let inventory = self.blockchain_manager.block_tree().active_blockchain();
        match self
            .network
            .send_to(sender, PeerMessage::ResponseInventory(inventory))
        {
            Ok(true) => Ok(()),
            Ok(false) => {
                // Flow control kicked in, we will store the request and send it later.
                self.outstanding_get_inventory_requests
                    .push(sender.to_string());
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn on_response_inventory(
        &mut self,
        _sender: &str,
        inventory: Vec<Block>,
        current_time: u32,
    ) -> Result<(), String> {
        // Skip the genesis block.
        for block in inventory.into_iter().skip(1) {
            self.new_block(block, current_time)?;
        }
        Ok(())
    }

    fn on_relay_block(
        &mut self,
        sender: &str,
        block: Block,
        current_time: u32,
    ) -> Result<(), String> {
        self.new_block(block.clone(), current_time)?;
        self.network
            .multicast(PeerMessage::RelayBlock(block), vec![sender.to_string()])
    }

    fn new_block(&mut self, block: Block, current_time: u32) -> Result<(), String> {
        self.blockchain_manager
            // TODO: If the validation fails, we should disconnect the peer.
            .on_block_received(block.clone(), current_time)
    }

    fn on_relay_transaction(
        &mut self,
        sender: &str,
        transaction: Transaction,
    ) -> Result<(), String> {
        // TODO: If validation fails, we should disconnect the peer.
        self.network.multicast(
            PeerMessage::RelayTransaction(transaction),
            vec![sender.to_string()],
        )
    }
}
