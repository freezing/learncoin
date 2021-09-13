use crate::Sha256;
use std::fmt::{Display, Formatter};

/// A double SHA-256 hash of the transaction data.
#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub struct TransactionId(Sha256);

impl Display for TransactionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TransactionId {
    pub fn new(data: Sha256) -> Self {
        Self(data)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0.as_slice()
    }
}

/// The index of the transaction output.
#[derive(Debug, Eq, PartialEq, Clone)]
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

#[derive(Debug, Clone)]
pub struct TransactionInput {
    // 32 bytes. A pointer to the transaction containing the UTXO to be spent.
    utxo_id: TransactionId,
    // 4 bytes. The number of UTXO to be spent, the first one is 0.
    output_index: OutputIndex,
    // TODO: Add unlocking script.
    // Transaction inputs must provide the unlocking script that is a solution to
    // the locking script in the reference transaction output.
    // This is required to implement validation.
}

impl Display for TransactionInput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.utxo_id, self.output_index)
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
}

#[derive(Debug, Clone)]
pub struct TransactionOutput {
    // TODO: Use a locking script.
    // A String is not really specified in the transaction output.
    // Instead, we should specify a locking script. The transaction input referencing this output
    // should provide the unlocking script to prove the ownership.
    // This is required to implement validation.
    // For the time being, we are going to store the String with the transaction output.
    to: String,
    amount: i64,
}

impl Display for TransactionOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.to, self.amount)
    }
}

impl TransactionOutput {
    pub fn new(to: String, amount: i64) -> Self {
        Self { to, amount }
    }

    pub fn to(&self) -> &String {
        &self.to
    }

    pub fn amount(&self) -> i64 {
        self.amount
    }
}

#[derive(Debug, Clone)]
pub struct Transaction {
    id: TransactionId,
    inputs: Vec<TransactionInput>,
    outputs: Vec<TransactionOutput>,
}

impl Transaction {
    pub fn new(
        inputs: Vec<TransactionInput>,
        outputs: Vec<TransactionOutput>,
    ) -> Result<Self, String> {
        let id = Self::hash_transaction_data(&inputs, &outputs);
        let transaction = Self {
            id,
            inputs,
            outputs,
        };
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
        TransactionId(Sha256::digest(data.as_bytes()))
    }
}
