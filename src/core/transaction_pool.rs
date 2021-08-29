use crate::core::transaction::TransactionId;
use crate::core::Transaction;
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

    /// Ensures that the transaction exists in the pool.
    pub fn insert(&mut self, transaction: Transaction) {
        self.transactions.insert(*transaction.id(), transaction);
    }

    // TODO: Process block and undo block (whenever an active chain is updated).
}
