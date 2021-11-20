/// Information about the peer tracked by the local node.
pub struct PeerState {
    pub expect_version_message: bool,
    pub expect_verack_message: bool,
}

impl PeerState {
    pub fn new() -> Self {
        Self {
            expect_version_message: false,
            expect_verack_message: false,
        }
    }
}
