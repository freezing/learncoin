use crate::core::hash::hash;
use crate::core::{Address, Coolcoin, Sha256};
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;
use std::fmt::{Display, Formatter};

big_array! {BigArray;}

/// A double SHA-256 hash of the transaction data.
#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub struct TransactionId(Sha256);

impl Display for TransactionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // TODO: Print as hex.
        write!(f, "{:#?}", self.0)
    }
}

impl TransactionId {
    pub fn new(data: Sha256) -> Self {
        Self(data)
    }
}

/// 4 bytes representing the index of the transaction output.
#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct OutputIndex(i32);

impl Display for OutputIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl OutputIndex {
    pub const fn new(index: i32) -> Self {
        Self(index)
    }
}

// Set all bits to 0.
const COINBASE_UTXO_ID: TransactionId = TransactionId([0; 32]);
// Set all bits to 1.
const COINBASE_OUTPUT_INDEX: OutputIndex = OutputIndex::new(-1);

// TODO: Coinbase transaction input has coinbase data size and coinbase data, which is
// arbitrary data used for extra nonce and mining tags.
// This is used instead of the unlocking script.
// Question: How to model this as an object?
// Potential solution: store encoded values as bytes, so this allows both to be modelled with
// the same data type. It is also how the actual bitcoin transaction is modelled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInput {
    // 32 bytes. A pointer to the transaction containing the UTXO to be spent.
    utxo_id: TransactionId,
    // 4 bytes. The number of the UTXO to be spent, first one is 0.
    output_index: OutputIndex,
    // TODO: Add unlocking script.
}

impl Display for TransactionInput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.utxo_id, self.output_index)
    }
}

impl TransactionInput {
    pub fn new(utxo_id: TransactionId, output_index: OutputIndex) -> Self {
        Self {
            utxo_id,
            output_index,
        }
    }

    pub fn output_index(&self) -> &OutputIndex {
        &self.output_index
    }
    pub fn utxo_id(&self) -> &TransactionId {
        &self.utxo_id
    }

    pub fn new_coinbase() -> Self {
        Self {
            utxo_id: COINBASE_UTXO_ID,
            output_index: COINBASE_OUTPUT_INDEX,
        }
    }

    pub fn is_coinbase(&self) -> bool {
        self.utxo_id == COINBASE_UTXO_ID && self.output_index == COINBASE_OUTPUT_INDEX
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionOutput {
    // TODO: Address is actually a locking script.
    to: Address,
    amount: Coolcoin,
}

impl Display for TransactionOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.to, self.amount)
    }
}

impl TransactionOutput {
    pub fn new(to: Address, amount: Coolcoin) -> Self {
        Self { to, amount }
    }

    pub fn to(&self) -> &Address {
        &self.to
    }

    pub fn amount(&self) -> Coolcoin {
        self.amount
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    id: TransactionId,
    inputs: Vec<TransactionInput>,
    outputs: Vec<TransactionOutput>,
    // A minimum block height that this transaction can be included in.
    // Used to avoid collisions when two transactions have the same inputs and outputs,
    // which is possible when inputs are a single coinbase transaction input and output is the
    // same address.
    locktime: u32,
}

impl Transaction {
    pub fn new(
        inputs: Vec<TransactionInput>,
        outputs: Vec<TransactionOutput>,
        locktime: u32,
    ) -> Result<Self, String> {
        let id = Self::hash_transaction_data(&inputs, &outputs);
        let transaction = Self {
            id,
            inputs,
            outputs,
            locktime,
        };
        transaction.validate_format()?;
        Ok(transaction)
    }

    pub fn id(&self) -> &TransactionId {
        &self.id
    }

    pub fn inputs(&self) -> &Vec<TransactionInput> {
        &self.inputs
    }

    pub fn outputs(&self) -> &Vec<TransactionOutput> {
        &self.outputs
    }

    pub fn is_coinbase(&self) -> bool {
        self.inputs.get(0).unwrap().is_coinbase()
    }

    /// Checks if the format of the transaction is valid, i.e.
    /// Format is valid if any of the following are satisfied:
    ///   - A transaction contains no coinbase inputs
    ///   - A transaction contains exactly 1 coinbase input and exactly one output.
    fn validate_format(&self) -> Result<(), String> {
        let contains_coinbase_inputs = self.inputs.iter().any(TransactionInput::is_coinbase);
        let coinbase_requirements_satisfied = self.inputs.len() == 1 && self.outputs.len() == 1;
        if contains_coinbase_inputs && !coinbase_requirements_satisfied {
            Err(format!("Transaction: {} has the coinbase input, but it doesn't satisfy all coinbase requirements.", self.id))
        } else {
            Ok(())
        }
    }

    fn hash_transaction_data(
        inputs: &Vec<TransactionInput>,
        outputs: &Vec<TransactionOutput>,
    ) -> TransactionId {
        let data = format!(
            "{}{}",
            inputs
                .iter()
                .map(TransactionInput::to_string)
                .collect::<Vec<String>>()
                .join(""),
            outputs
                .iter()
                .map(TransactionOutput::to_string)
                .collect::<Vec<String>>()
                .join("")
        );
        TransactionId(hash(data.as_bytes()))
    }
}
