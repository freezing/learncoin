use crate::core::{Address, Coolcoin};
use std::fmt::{Display, Formatter};

pub struct OutputIndex(usize);
/// A hash of the transaction.
pub struct TransactionId(String);

impl Display for TransactionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for TransactionId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

// TODO: Coinbase transaction input has coinbase data size and coinbase data, which is
// arbitrary data used for extra nonce and mining tags.
// This is used instead of the unlocking script.
// Question: How to model this as an object?
// Potential solution: store encoded values as bytes, so this allows both to be modelled with
// the same data type. It is also how the actual bitcoin transaction is modelled.
pub struct TransactionInput {
    // 32 bytes. A pointer to the transaction containing the UTXO to be spent.
    transaction_id: TransactionId,
    // 4 bytes. The number of the UTXO to eb spent, first one is 0.
    output_index: OutputIndex,
    // TODO: Add unlocking script.
}

impl TransactionInput {
    pub fn new(transaction_id: TransactionId, output_index: OutputIndex) -> Self {
        Self {
            transaction_id,
            output_index,
        }
    }

    pub fn transaction_id(&self) -> &TransactionId {
        &self.transaction_id
    }
    pub fn output_index(&self) -> &OutputIndex {
        &self.output_index
    }
    pub fn amount(&self) -> Coolcoin {
        self.amount
    }
}

pub struct TransactionOutput {
    // TODO: Address is actually a locking script.
    to: Address,
    amount: Coolcoin,
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

pub struct Transaction {
    id: TransactionId,
    inputs: Vec<TransactionInput>,
    outputs: Vec<TransactionOutput>,
}

impl Transaction {
    pub fn new(
        id: TransactionId,
        inputs: Vec<TransactionInput>,
        outputs: Vec<TransactionOutput>,
    ) -> Self {
        Self {
            id,
            inputs,
            outputs,
        }
    }

    pub fn fee(&self) -> Coolcoin {
        self.inputs
            .iter()
            .map(TransactionInput::amount)
            .sum::<Coolcoin>()
            - self
                .outputs
                .iter()
                .map(TransactionOutput::amount)
                .sum::<Coolcoin>()
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
}
