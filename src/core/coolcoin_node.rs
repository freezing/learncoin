use crate::core::{BlockchainManager, CoolcoinNetwork};
use std::net::TcpStream;
use std::sync::Arc;

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
            // Receive data from the network.
            let data = self.network.receive_data();
        }
        todo!()
    }
}
