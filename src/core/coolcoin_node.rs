use crate::core::block::BlockHash;
use crate::core::coolcoin_network::NetworkParams;
use crate::core::miner::{Miner, MinerRequest, MinerResponse};
use crate::core::peer_connection::PeerMessage;
use crate::core::{
    Block, BlockchainManager, ChainContext, CoolcoinNetwork, Transaction, TransactionPool,
    UtxoContext, UtxoPool,
};
use std::net::TcpStream;
use std::sync::mpsc::TryRecvError;
use std::sync::Arc;
use std::thread;
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
    transaction_pool: TransactionPool,
    utxo_pool: UtxoPool,
}

impl CoolcoinNode {
    pub fn connect(network_params: NetworkParams) -> Result<Self, String> {
        let network = CoolcoinNetwork::connect(&network_params)?;
        Ok(Self {
            network,
            blockchain_manager: BlockchainManager::new(),
            outstanding_get_inventory_requests: Vec::new(),
            transaction_pool: TransactionPool::new(),
            utxo_pool: UtxoPool::new(),
        })
    }

    pub fn run(mut self) {
        // If we can't send messages to all nodes immediately, then there is no point in trying
        // to recover since this is part of the startup.
        // It is okay for the process to fail since retrying would mean rerunning the process.
        // Of course, in production like implementation we would handle that in code.
        self.network.broadcast(PeerMessage::GetInventory()).unwrap();

        let mut miner = Miner::start_async();

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

            // Update miner and check if there are any new blocks.
            match miner.read() {
                Ok(MinerResponse::None(request)) => {
                    println!("Miner failed to mine a block for request: {:#?}", request);
                }
                Ok(MinerResponse::Mined(block)) => {
                    println!(
                        "Miner has successfully mined a new block: {}",
                        serde_json::to_string_pretty(&block).unwrap()
                    );
                    self.process_new_block_and_update_active_blockchain(block);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    eprintln!("Miner has been disconnected!")
                }
            }

            if miner.num_outstanding_requests() == 0 && !self.transaction_pool.is_empty() {
                let previous_block_hash = self.blockchain_manager.tip().clone();
                let transactions = self.transaction_pool.all().clone();
                // TODO: Difficulty target should be returned by the blockchain manager,
                // and it should be adjusted for each chain.
                let difficulty_target = self
                    .blockchain_manager
                    .block_tree()
                    .get(self.blockchain_manager.tip())
                    .unwrap()
                    .header()
                    .difficulty_target();
                match miner.send(MinerRequest::new(
                    previous_block_hash,
                    transactions,
                    difficulty_target,
                )) {
                    Ok(()) => {
                        println!("Requested from miner to mine block.");
                    }
                    Err(e) => {
                        eprintln!("{}", e.to_string());
                    }
                }
            }

            thread::sleep(Duration::from_millis(100));
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
            PeerMessage::RelayBlock(block) => self.on_relay_block(sender, block),
            PeerMessage::RelayTransaction(transaction) => {
                self.on_relay_transaction(sender, transaction)
            }
            PeerMessage::GetBlock(block_hash) => self.on_get_block(sender, block_hash),
            PeerMessage::ResponseBlock(_block) => {
                todo!()
            }
            PeerMessage::SendTransaction(transaction) => {
                self.on_send_transaction(sender, transaction)
            }
            PeerMessage::ResponseTransaction => {
                todo!()
            }
            PeerMessage::GetFullBlockchain => self.on_get_full_blockchain(sender),
            PeerMessage::ResponseFullBlockchain(_blocks) => {
                todo!()
            }
        }
    }

    fn on_get_full_blockchain(&mut self, sender: &str) -> Result<(), String> {
        let blocks = self.blockchain_manager.all_blocks();
        self.network
            .send_to(sender, PeerMessage::ResponseFullBlockchain(blocks))?;
        Ok(())
    }

    fn on_get_block(&mut self, sender: &str, block_hash: BlockHash) -> Result<(), String> {
        let block = self
            .blockchain_manager
            .block_tree()
            .get(&block_hash)
            .map(|b| b.clone());
        self.network
            .send_to(sender, PeerMessage::ResponseBlock(block))?;
        Ok(())
    }

    fn on_send_transaction(
        &mut self,
        sender: &str,
        transaction: Transaction,
    ) -> Result<(), String> {
        self.on_new_transaction(sender, transaction)?;
        self.network
            .send_to(sender, PeerMessage::ResponseTransaction)?;
        Ok(())
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
        _current_time: u32,
    ) -> Result<(), String> {
        // Skip the genesis block.
        for block in inventory.into_iter().skip(1) {
            self.process_new_block_and_update_active_blockchain(block)?;
        }
        Ok(())
    }

    fn on_relay_block(&mut self, _sender: &str, block: Block) -> Result<(), String> {
        self.process_new_block_and_update_active_blockchain(block)
    }

    fn process_new_block_and_update_active_blockchain(
        &mut self,
        block: Block,
    ) -> Result<(), String> {
        let old_tip = self.blockchain_manager.tip().clone();
        self.process_new_block(block)?;
        let new_tip = self.blockchain_manager.tip().clone();
        self.on_active_blockchain_changed(&old_tip, &new_tip);
        Ok(())
    }

    /// Should only be called by process_new_block_and_update_active_blockchain
    fn process_new_block(&mut self, block: Block) -> Result<(), String> {
        // TODO: This method is useful for client as well, extract it as a library.
        if self.blockchain_manager.exists(&block) {
            Ok(())
        } else {
            let orphans = self.blockchain_manager.new_block(block.clone());
            // Broadcast is fine here because the sender would drop it given that it already
            // has it.
            self.network.broadcast(PeerMessage::RelayBlock(block));

            // TODO: Validate block.
            // TODO: If the validation fails, we should disconnect the peer.
            let mut errors = vec![];
            for orphan in orphans {
                match self.process_new_block(orphan) {
                    Ok(()) => {}
                    Err(e) => errors.push(e),
                }
            }

            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors.join("\n"))
            }
        }
    }

    fn on_relay_transaction(
        &mut self,
        sender: &str,
        transaction: Transaction,
    ) -> Result<(), String> {
        self.on_new_transaction(sender, transaction)
    }

    fn on_new_transaction(&mut self, sender: &str, transaction: Transaction) -> Result<(), String> {
        // TODO: If validation fails, we should disconnect the peers and do not insert it.
        self.transaction_pool.insert(transaction.clone());
        self.network.multicast(
            PeerMessage::RelayTransaction(transaction),
            vec![sender.to_string()],
        )
    }

    fn on_active_blockchain_changed(&mut self, old_tip: &BlockHash, new_tip: &BlockHash) {
        // The fork is always expected to exist at this stage because only the nodes with a
        // parent have been inserted in the block tree.
        // If fork block is the same as old_tip, then this is an extension of the already active
        // block chain, so no old blocks need to be deleted.
        // As a matter of fact, we don't have to special-case this scenario because the old path
        // would be empty since it doesn't include the fork.
        // TODO: Write a unit test to ensure this is correct.
        let (_fork, path_old, path_new) = self
            .blockchain_manager
            .block_tree()
            .find_fork(old_tip, new_tip)
            .unwrap();

        for old_block in &path_old {
            self.transaction_pool
                // TODO: Fork should return full blocks not just hash.
                .undo_active_block(self.blockchain_manager.block_tree().get(old_block).unwrap());
        }

        for new_block in &path_new {
            self.transaction_pool
                .new_active_block(self.blockchain_manager.block_tree().get(new_block).unwrap());
        }
    }

    // Below are required for validation.
    fn fetch_chain_context(&self, _block: &Block) -> ChainContext {
        todo!()
    }

    fn fetch_utxo_context(&self, _block: &Block) -> UtxoContext {
        todo!()
    }

    fn update_utxo_pool(&self) {
        todo!("Handle UTXO pool")
    }
}
