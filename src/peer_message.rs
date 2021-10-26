use crate::{Block, BlockHash};
use serde::{Deserialize, Serialize};

/// Metadata about the MessagePayload.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PeerMessageHeader {
    payload_size: u32,
}

impl PeerMessageHeader {
    pub const SIZE: usize = std::mem::size_of::<PeerMessageHeader>();

    pub fn new(payload_size: u32) -> Self {
        Self { payload_size }
    }

    pub fn payload_size(&self) -> u32 {
        self.payload_size
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct VersionMessage {
    version: u32,
}

impl VersionMessage {
    pub fn new(version: u32) -> Self {
        Self { version }
    }

    pub fn version(&self) -> u32 {
        self.version
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct GetBlocks {
    // Sorted in descending order by the block height.
    block_locator_hashes: Vec<BlockHash>,
}

impl GetBlocks {
    pub fn new(block_locator_hashes: Vec<BlockHash>) -> Self {
        Self {
            block_locator_hashes,
        }
    }

    pub fn block_locator_hashes(&self) -> &Vec<BlockHash> {
        &self.block_locator_hashes
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Inventory {
    // Bitcoin protocol uses Inventory message to send any kind of hashes.
    // However, in the learncoin implementation, we only use it to send block hashes.
    hashes: Vec<BlockHash>,
}

impl Inventory {
    pub fn new(hashes: Vec<BlockHash>) -> Self {
        Self { hashes }
    }

    pub fn hashes(&self) -> &Vec<BlockHash> {
        &self.hashes
    }

    pub fn into_hashes(self) -> Vec<BlockHash> {
        self.hashes
    }
}

/// Payload sent to and received from the peers.
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum PeerMessagePayload {
    Version(VersionMessage),
    Verack,
    GetBlocks(GetBlocks),
    Inv(Inventory),
    GetData(BlockHash),
    Block(Block),
}

/// An API to encode and decode peer messages.
pub trait PeerMessageEncoding<T> {
    /// Encodes the message into the buffer.
    /// Returns a successful result, or a string describing the error.
    fn encode(&self, buffer: &mut [u8]) -> Result<(), String>;

    /// Returns the size of the encoded message.
    fn encoded_size(&self) -> Result<u64, String>;

    /// Decodes the message from the buffer.
    /// Returns the decoded message or a string describing the error.
    fn decode(buffer: &[u8]) -> Result<T, String>;
}

impl PeerMessageEncoding<PeerMessageHeader> for PeerMessageHeader {
    fn encode(&self, buffer: &mut [u8]) -> Result<(), String> {
        bincode::serialize_into(buffer, self).map_err(|e| e.to_string())
    }

    fn encoded_size(&self) -> Result<u64, String> {
        bincode::serialized_size(self).map_err(|e| e.to_string())
    }

    fn decode(buffer: &[u8]) -> Result<Self, String> {
        bincode::deserialize::<Self>(buffer).map_err(|e| e.to_string())
    }
}

impl PeerMessageEncoding<PeerMessagePayload> for PeerMessagePayload {
    fn encode(&self, buffer: &mut [u8]) -> Result<(), String> {
        bincode::serialize_into(buffer, self).map_err(|e| e.to_string())
    }

    fn encoded_size(&self) -> Result<u64, String> {
        bincode::serialized_size(self).map_err(|e| e.to_string())
    }

    fn decode(buffer: &[u8]) -> Result<Self, String> {
        bincode::deserialize::<Self>(buffer).map_err(|e| e.to_string())
    }
}
