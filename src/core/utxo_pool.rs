use crate::core::transaction::{OutputIndex, TransactionId, TransactionOutput};
use std::collections::HashMap;

/// A pool of confirmed and unspent transaction outputs.
pub struct UtxoPool {
    // Unspent transaction outputs, indexed by their transaction ID and their index in the
    // transaction.
    utxos: HashMap<(TransactionId, OutputIndex), TransactionOutput>,
}

impl UtxoPool {
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
        }
    }
}
