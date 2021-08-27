use crate::core::{Address, Coolcoin};
use std::fmt::{Display, Formatter};

pub struct OutputIndex(usize);
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

pub struct TransactionInput {
    transaction_id: TransactionId,
    output_index: OutputIndex,
    amount: Coolcoin,
}

impl TransactionInput {
    pub fn new(transaction_id: TransactionId, output_index: OutputIndex, amount: Coolcoin) -> Self {
        Self {
            transaction_id,
            output_index,
            amount,
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
