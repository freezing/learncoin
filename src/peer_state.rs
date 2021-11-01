use std::time::Instant;

/// Information about the peer tracked by the local node.
pub struct PeerState {
    pub expect_version_message: bool,
    pub expect_verack_message: bool,
    pub headers_message_sent_at: Option<Instant>,
}

impl PeerState {
    pub fn new() -> Self {
        Self {
            expect_version_message: false,
            expect_verack_message: false,
            headers_message_sent_at: None,
        }
    }
}
