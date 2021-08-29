use crate::core::block::BlockHash;
use crate::core::{target_hash, Block};
use std::cmp::Ordering;

pub struct UtxoContext {}
pub struct ChainContext {
    target_hash: BlockHash,
}
// Responsible for performing validation checks on the block.
// Note that this is a non-exhaustive list of checks.
// The real blockchain implementation would have more checks, e.g.
// the block data structure is syntactically valid, block size is within acceptable limits,
// etc.
pub struct BlockValidator {}

impl BlockValidator {
    pub fn validate_no_context(block: &Block, current_time: u32) -> Result<(), String> {
        Self::validate_timestamp_less_than_two_hours_in_the_future(
            block.header().timestamp(),
            current_time,
        )?;
        Self::validate_only_first_transaction_is_coinbase(&block)?;
        Self::validate_header_hash_less_than_target(
            &block.header().hash(),
            &target_hash(block.header().difficulty_target()),
        )
    }

    pub fn validate_chain_context(
        block: &Block,
        chain_context: &ChainContext,
        _current_time: u32,
    ) -> Result<(), String> {
        Self::validate_header_hash_less_than_target(
            &block.header().hash(),
            &chain_context.target_hash,
        )?;
        Ok(())
    }

    pub fn validate_utxo_context(block: &Block, utxo_context: &UtxoContext) -> Result<(), String> {
        Self::validate_all_transactions_are_valid(&block, &utxo_context)
    }

    fn validate_header_hash_less_than_target(
        header_hash: &BlockHash,
        target_hash: &BlockHash,
    ) -> Result<(), String> {
        match header_hash.cmp(target_hash) {
            Ordering::Less => Ok(()),
            Ordering::Equal | Ordering::Greater => Err(format!(
                "Header hash: {} is not less than target hash: {}",
                header_hash, target_hash
            )),
        }
    }

    fn validate_timestamp_less_than_two_hours_in_the_future(
        header_timestamp: u32,
        current_timestamp: u32,
    ) -> Result<(), String> {
        const TWO_HOURS_IN_SECONDS: i64 = 2 * 60 * 60;
        if (current_timestamp as i64 - header_timestamp as i64).abs() < TWO_HOURS_IN_SECONDS {
            Ok(())
        } else {
            Err(format!(
                "Header timestamp: {} is not within 2 hours of current timestamp: {}",
                header_timestamp, current_timestamp
            ))
        }
    }

    fn validate_only_first_transaction_is_coinbase(block: &Block) -> Result<(), String> {
        if block.transactions().is_empty() {
            Err(format!(
                "No transactions found in block: {}",
                block.header().hash()
            ))
        } else if block
            .transactions()
            .iter()
            .enumerate()
            .any(|(idx, transaction)| idx != 0 && transaction.is_coinbase())
        {
            Err(format!(
                "Block: {} contains transactions at index > 0 that are coinbase.",
                block.header().hash()
            ))
        } else {
            Ok(())
        }
    }

    fn validate_all_transactions_are_valid(
        _block: &Block,
        _utxo_context: &UtxoContext,
    ) -> Result<(), String> {
        todo!("Transaction validation requires UtxoDatabase to find total coins in inputs")
    }
}
