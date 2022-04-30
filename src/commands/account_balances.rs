use crate::{
    Block, OutputIndex, PublicKey, Transaction, TransactionId, TransactionInput, TransactionOutput,
};
use std::collections::HashMap;

pub struct AccountBalances {}

impl AccountBalances {
    pub fn extract_account_balances(active_blocks: &Vec<Block>) -> HashMap<PublicKey, i64> {
        let mut transactions = HashMap::new();
        let mut balances = HashMap::new();
        for block in active_blocks {
            for transaction in block.transactions() {
                transactions.insert(*transaction.id(), transaction);
                for input in transaction.inputs() {
                    Self::process_input(&transactions, &mut balances, input);
                }
                for output in transaction.outputs() {
                    Self::process_output(&mut balances, output);
                }
            }
        }
        balances
    }

    fn process_input(
        transactions: &HashMap<TransactionId, &Transaction>,
        balances: &mut HashMap<PublicKey, i64>,
        input: &TransactionInput,
    ) {
        // Decrement the balance for spent transaction output.
        // Coinbase inputs do not take money from any transaction output, so we ignore
        // those.
        if !input.is_coinbase() {
            // Spend the referenced transaction output.
            let txid = input.utxo_id();
            match transactions.get(txid) {
                None => {
                    panic!("Invalid blockchain. Unknown transaction: {}", txid)
                }
                Some(transaction) => {
                    match transaction
                        .outputs()
                        .get(input.output_index().value() as usize)
                    {
                        None => {
                            panic!(
                                "Invalid blockchain. Unknown transaction output at index: {}",
                                input.output_index()
                            )
                        }
                        Some(output) => {
                            let public_key = output.locking_script().public_key();
                            // Safety: We must have already processed the TransactionOutput,
                            // so it's fine to assume that the key exists.
                            let mut balance = balances.get_mut(public_key).unwrap();
                            *balance -= output.amount();
                        }
                    }
                }
            }
        }
    }

    fn process_output(balances: &mut HashMap<PublicKey, i64>, output: &TransactionOutput) {
        // Increment the balance for unspent transaction output.
        let public_key = output.locking_script().public_key().clone();
        // Ensure that the key exists if it's the first time we're seeing the account address.
        let balance = balances.entry(public_key).or_insert(0);
        *balance += output.amount();
    }
}
