use crate::{Block, OutputIndex, Transaction, TransactionId, TransactionOutput};
use std::collections::{HashMap, HashSet};

pub struct Transactions {}

impl Transactions {
    pub fn extract_transaction_outputs(
        active_blockchain: &Vec<Block>,
        utxo_only: bool,
    ) -> Vec<(TransactionId, OutputIndex, TransactionOutput)> {
        let mut transactions = HashMap::new();
        let mut outputs = HashMap::new();
        let mut spent_outputs = HashSet::new();
        for block in active_blockchain {
            for transaction in block.transactions() {
                transactions.insert(*transaction.id(), transaction.clone());
                for input in transaction.inputs() {
                    spent_outputs.insert((input.utxo_id(), input.output_index()));
                }
                for (index, output) in transaction.outputs().iter().enumerate() {
                    let output_index = OutputIndex::new(index as i32);
                    outputs.insert((*transaction.id(), output_index), output.clone());
                }
            }
        }
        outputs
            .iter()
            .filter(|((transaction_id, output_index), output)| {
                !utxo_only || !spent_outputs.contains(&(transaction_id, output_index))
            })
            .map(|((transaction_id, output_index), output)| {
                (*transaction_id, *output_index, output.clone())
            })
            .collect()
    }

    pub fn extract_transaction(
        active_blockchain: &Vec<Block>,
        id: TransactionId,
    ) -> Result<Option<(Transaction, u32)>, String> {
        let mut result: Option<(Transaction, u32)> = None;
        for block in active_blockchain {
            match &mut result {
                None => {}
                Some((_, ref mut confirmations)) => {
                    *confirmations += 1;
                }
            }

            for transaction in block.transactions() {
                if *transaction.id() == id {
                    if result.is_none() {
                        result = Some((transaction.clone(), 1))
                    } else {
                        return Err(format!(
                            "Duplicate transaction ID on the blockchain: {}",
                            transaction.id()
                        ));
                    }
                }
            }
        }
        Ok(result)
    }
}
