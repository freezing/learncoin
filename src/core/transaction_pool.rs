use crate::core::transaction::TransactionId;
use crate::core::{Block, Transaction};
use std::collections::HashMap;

/// An unordered collection of transactions that are not in blocks in the main chain,
/// but for which we have input transactions.
/// Note that each node may have a different transaction pool since this is not maintained
/// from the genesis block.
/// Instead, it only contains the transactions received from the network since the node
/// was started.
pub struct TransactionPool {
    transactions: HashMap<TransactionId, Transaction>,
}

impl TransactionPool {
    pub fn new() -> Self {
        Self {
            transactions: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    pub fn all(&self) -> Vec<Transaction> {
        self.transactions.values().map(|t| t.clone()).collect()
    }

    /// Ensures that the transaction exists in the pool.
    pub fn insert(&mut self, transaction: Transaction) {
        self.transactions.insert(*transaction.id(), transaction);
    }

    pub fn new_active_block(&mut self, block: &Block) {
        for transaction in block.transactions() {
            self.transactions.remove(transaction.id());
            // Previous transaction may not exist, e.g. because the node was started later.
        }
    }

    pub fn undo_active_block(&mut self, block: &Block) {
        let transactions = block.transactions().to_vec();
        for transaction in transactions {
            let previous = self.transactions.insert(*transaction.id(), transaction);
            assert!(previous.is_some());
        }
    }
}
