use crate::{FlipBuffer, PeerMessageEncoding, PeerMessageHeader, PeerMessagePayload};
use std::fmt::Debug;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};

pub struct MessageLogger {}

impl MessageLogger {
    pub fn log<T: Debug>(prefix: &str, message: &T) {
        println!("{} {:#?}", prefix, message);
    }
}

/// A TCP connection to the peer in the LearnCoin network.
pub struct PeerConnection {
    peer_address: String,
    tcp_stream: TcpStream,
    // An implementation detail of the receive method.
    buffer: FlipBuffer,
}

impl PeerConnection {
    /// Establishes a TCP connection with a peer at the given address.
    pub fn connect(peer_address: String, recv_buffer_size: usize) -> Result<Self, String> {
        let tcp_stream = TcpStream::connect(&peer_address).map_err(|e| e.to_string())?;
        tcp_stream
            .set_nonblocking(true)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            peer_address,
            tcp_stream,
            buffer: FlipBuffer::new(recv_buffer_size),
        })
    }

    /// Creates a PeerConnection from the already established TCP connection.
    /// A common use-case is a TCP connection with a peer established by listening for
    /// new TCP connections.
    pub fn from_established_tcp(
        address: SocketAddr,
        tcp_stream: TcpStream,
        recv_buffer_size: usize,
    ) -> Self {
        Self {
            peer_address: address.to_string(),
            tcp_stream,
            buffer: FlipBuffer::new(recv_buffer_size),
        }
    }

    pub fn peer_address(&self) -> &str {
        &self.peer_address
    }

    /// Sends the given payload to the peer.
    /// Returns true if the payload has been sent successfully or false if the call would block.
    /// The call would block if the underlying TCP socket is full, and the peer can't receive more
    /// data due to flow control.
    pub fn send(&mut self, payload: &PeerMessagePayload) -> Result<bool, String> {
        let header_size = std::mem::size_of::<PeerMessageHeader>();
        let payload_size = payload.encoded_size()? as usize;
        let total_size = header_size + payload_size as usize;
        let header = PeerMessageHeader::new(payload_size as u32);

        MessageLogger::log("Send:", &payload);

        let mut buffer = Self::allocate_buffer(total_size);
        header.encode(&mut buffer[..header_size])?;
        payload.encode(&mut buffer[header_size..])?;

        match self.tcp_stream.write(&buffer[..]) {
            Ok(0) => {
                // TcpStream::write returns zero when the received is unlikely to be able to
                // receive more bytes, e.g. the connection may be shutdown.
                Err(format!(
                    "Connection to peer: {} has been lost. Sent 0 bytes.",
                    self.peer_address
                ))
            }
            Ok(_) => Ok(true),
            Err(e) => match e.kind() {
                ErrorKind::WouldBlock => Ok(false),
                _ => Err(e.to_string()),
            },
        }
    }

    /// Attempts to read a new payload from the peer.
    /// Returns Ok(Some(payload)) if the payload exists, Ok(None) if there are no new payloads,
    /// or a string describing the error.
    /// For example, an error may happen if the connection to the peer has been lost.
    pub fn receive(&mut self) -> Result<Option<PeerMessagePayload>, String> {
        // Ensure that the buffer data is at the beginning so it can never overflow.
        self.buffer.flip();

        self.read()?;
        match self.decode_header()? {
            None => Ok(None),
            Some(header) => match self.decode_payload(header.payload_size())? {
                None => Ok(None),
                Some(payload) => {
                    MessageLogger::log("Recv:", &payload);
                    // Now that we have decoded the payload, we can drop the used data from
                    // the buffer.
                    self.buffer
                        .consume_data(PeerMessageHeader::SIZE + header.payload_size() as usize);
                    Ok(Some(payload))
                }
            },
        }
    }

    /// Equivalent to calling `receive` until it returns Ok(None) or Err.
    pub fn receive_all(&mut self) -> Result<Vec<PeerMessagePayload>, String> {
        let mut messages = vec![];
        loop {
            match self.receive() {
                Ok(Some(message)) => messages.push(message),
                Ok(None) => break,
                Err(e) => return Err(e),
            }
        }
        Ok(messages)
    }

    fn read(&mut self) -> Result<(), String> {
        match self.tcp_stream.read(self.buffer.free_space_slice_mut()) {
            Ok(0) => {
                // TcpStream::read returns zero when the connection is shutdown.
                Err(format!(
                    "Connection to peer: {} has been lost. Received 0 bytes.",
                    self.peer_address
                ))
            }
            Ok(read_bytes) => {
                self.buffer.consume_free_space(read_bytes);
                Ok(())
            }
            Err(e) => match e.kind() {
                ErrorKind::WouldBlock => Ok(()),
                _ => Err(e.to_string()),
            },
        }
    }

    fn decode_header(&mut self) -> Result<Option<PeerMessageHeader>, String> {
        self.decode_message(0, PeerMessageHeader::SIZE)
    }

    fn decode_payload(&mut self, payload_size: u32) -> Result<Option<PeerMessagePayload>, String> {
        self.decode_message(PeerMessageHeader::SIZE, payload_size as usize)
    }

    fn decode_message<T: PeerMessageEncoding<T>>(
        &mut self,
        offset: usize,
        message_size: usize,
    ) -> Result<Option<T>, String> {
        let data = &self.buffer.data()[offset..];
        if message_size > data.len() {
            // Not enough data.
            Ok(None)
        } else {
            T::decode(&data[..message_size]).map(|t| Some(t))
        }
    }

    fn allocate_buffer(size: usize) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(size);
        buffer.resize(size, 0);
        buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VersionMessage;

    #[test]
    fn encode_decode_header() {
        let header = PeerMessageHeader::new(8);
        let mut buffer = Vec::new();
        buffer.resize(PeerMessageHeader::SIZE, 0);
        header.encode(&mut buffer[..]).unwrap();

        let decoded = PeerMessageHeader::decode(&buffer[..]).unwrap();
        assert_eq!(decoded, header);
    }

    #[test]
    fn encode_decode_payload() {
        let payload = PeerMessagePayload::Version(VersionMessage::new(4));
        let payload_size = PeerMessagePayload::encoded_size(&payload).unwrap() as usize;
        let mut buffer = Vec::new();
        buffer.resize(payload_size, 0);
        payload.encode(&mut buffer[..]).unwrap();

        let decoded = PeerMessagePayload::decode(&buffer[..]).unwrap();
        assert_eq!(decoded, payload);
    }
}
