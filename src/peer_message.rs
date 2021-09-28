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

/// Payload sent to and received from the peers.
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum PeerMessagePayload {
    PlaceholderUntilWeImplementProtocol,
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
