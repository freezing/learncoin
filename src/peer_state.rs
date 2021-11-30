use std::time::Instant;

use crate::BlockHash;

/// Information about the peer tracked by the local node.
pub struct PeerState {
    pub expect_version_message: bool,
    pub expect_verack_message: bool,
    pub headers_message_sent_at: Option<Instant>,
    pub last_known_hash: BlockHash,
    pub last_common_block: BlockHash,
    pub num_blocks_in_transit: usize,
}

impl PeerState {
    pub fn new(genesis_hash: BlockHash) -> Self {
        Self {
            expect_version_message: false,
            expect_verack_message: false,
            headers_message_sent_at: None,
            last_known_hash: genesis_hash,
            last_common_block: genesis_hash,
            num_blocks_in_transit: 0,
        }
    }
}
