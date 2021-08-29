use crate::core::transaction::TransactionId;
use crate::core::Transaction;
use std::collections::HashMap;

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
}
