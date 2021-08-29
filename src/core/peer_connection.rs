use serde::{Deserialize, Serialize};
use std::io::{Error, ErrorKind, Read, Write};
use std::net::TcpStream;

#[derive(Serialize, Deserialize)]
struct PeerMessageHeader {
    payload_size: u32,
}

#[derive(Serialize, Deserialize)]
pub enum PeerMessage {
    GetBlocks(u32),
}

pub struct PeerConnection {
    peer_address: String,
    tcp_stream: TcpStream,
}

impl PeerConnection {
    pub fn connect(peer_address: String) -> Result<Self, String> {
        let mut tcp_stream = TcpStream::connect(&peer_address).map_err(|e| e.to_string())?;
        tcp_stream
            .set_nonblocking(true)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            peer_address,
            tcp_stream,
        })
    }

    pub fn receive(&mut self) -> Result<Option<PeerMessage>, String> {
        // Read header then read message.
        let header_size = std::mem::size_of::<PeerMessageHeader>();
        let mut header_buffer = Vec::with_capacity(header_size);

        let header = match self.tcp_stream.read(&mut header_buffer[..]) {
            Ok(read_bytes) => {
                assert_eq!(read_bytes, header_size);
                bincode::deserialize::<PeerMessageHeader>(&header_buffer).unwrap()
            }
            Err(e) => match e.kind() {
                ErrorKind::WouldBlock => return Ok(None),
                _ => return Err(e.to_string()),
            },
        };

        let mut payload_buffer = Vec::with_capacity(header.payload_size as usize);
        let payload = match self.tcp_stream.read(&mut payload_buffer[..]) {
            Ok(read_bytes) => {
                assert_eq!(read_bytes as u32, header.payload_size);
                bincode::deserialize::<PeerMessage>(&payload_buffer).unwrap()
            }
            Err(e) => match e.kind() {
                ErrorKind::WouldBlock => {
                    // We are not handling this case properly because if it blocks at this stage
                    // we would lose the header.
                    // Let's ignore it for now since it is not going to happen often in practice.
                    panic!("Unhandled would block");
                }
                _ => return Err(e.to_string()),
            },
        };

        Ok(Some(payload))
    }

    pub fn send(&mut self, payload: &PeerMessage) -> Result<bool, String> {
        let header_size = std::mem::size_of::<PeerMessageHeader>();
        let payload_size = bincode::serialized_size(&payload).unwrap() as usize;
        let total_size = header_size + payload_size;

        let mut buffer = Vec::with_capacity(total_size as usize);
        bincode::serialize_into(
            &mut buffer[..header_size],
            &PeerMessageHeader {
                payload_size: payload_size as u32,
            },
        )
        .unwrap();
        bincode::serialize_into(&mut buffer[header_size..], payload).unwrap();

        match self.tcp_stream.write(&buffer[..]) {
            Ok(_) => Ok(true),
            Err(e) => match e.kind() {
                ErrorKind::WouldBlock => Ok(false),
                _ => Err(e.to_string()),
            },
        }
    }
}
