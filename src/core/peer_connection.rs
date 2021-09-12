use crate::core::block::BlockHash;
use crate::core::{Block, Transaction};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::io::{Error, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};

macro_rules! log_info {
    () => (println!());
    ($($arg:tt)*) => ({
        println!($($arg)*);
    })
}

#[derive(Copy, Clone, Serialize, Deserialize)]
struct PeerMessageHeader {
    payload_size: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum PeerMessage {
    GetInventory(),
    ResponseInventory(Vec<Block>),
    GetBlock(BlockHash),
    ResponseBlock(Option<Block>),
    SendTransaction(Transaction),
    GetFullBlockchain,
    ResponseFullBlockchain(Vec<BlockHash>, Vec<Block>),
    ResponseTransaction,
    RelayBlock(Block),
    RelayTransaction(Transaction),
}

pub struct PeerConnection {
    peer_address: String,
    enable_logging: bool,
    tcp_stream: TcpStream,
    last_header: Option<PeerMessageHeader>,
}

impl PeerConnection {
    pub fn connect(peer_address: String, enable_logging: bool) -> Result<Self, String> {
        let mut tcp_stream = TcpStream::connect(&peer_address).map_err(|e| e.to_string())?;
        tcp_stream
            .set_nonblocking(true)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            peer_address,
            enable_logging,
            tcp_stream,
            last_header: None,
        })
    }

    pub fn address(&self) -> &str {
        &self.peer_address
    }

    pub fn from_tcp_stream(
        address: SocketAddr,
        tcp_stream: TcpStream,
        enable_logging: bool,
    ) -> Self {
        Self {
            peer_address: address.to_string(),
            enable_logging,
            tcp_stream,
            last_header: None,
        }
    }

    pub fn receive(&mut self) -> Result<Option<PeerMessage>, String> {
        // Read header then read message.
        let header_size = std::mem::size_of::<PeerMessageHeader>();
        let mut header_buffer = Vec::with_capacity(header_size);
        header_buffer.resize(header_size, 0);

        let header: PeerMessageHeader = match &self.last_header {
            Some(header) => header.clone(),
            None => match self.tcp_stream.read(&mut header_buffer[..]) {
                Ok(0) => {
                    // TcpStream::read returns zero when the connection is shutdown.
                    return Err(format!(
                        "Connection to peer: {} has been lost.",
                        self.peer_address
                    ));
                }
                Ok(read_bytes) => {
                    // TODO: Handle malicious peers.
                    assert_eq!(read_bytes, header_size);
                    bincode::deserialize::<PeerMessageHeader>(&header_buffer).unwrap()
                }
                Err(e) => match e.kind() {
                    // TODO: Consider dropping the peer if it would block.
                    ErrorKind::WouldBlock => return Ok(None),
                    _ => return Err(e.to_string()),
                },
            },
        };

        let mut payload_buffer = Vec::with_capacity(header.payload_size as usize);
        payload_buffer.resize(header.payload_size as usize, 0);
        let payload = match self.tcp_stream.read(&mut payload_buffer[..]) {
            Ok(read_bytes) => {
                assert_eq!(read_bytes as u32, header.payload_size);
                bincode::deserialize::<PeerMessage>(&payload_buffer).unwrap()
            }
            Err(e) => {
                return match e.kind() {
                    ErrorKind::WouldBlock => {
                        self.last_header = Some(header);
                        Ok(None)
                    }
                    _ => Err(e.to_string()),
                }
            }
        };
        self.last_header = None;
        if self.enable_logging {
            log_info!(
                "Recv [{}] {}",
                self.peer_address,
                serde_json::to_string_pretty(&payload).unwrap()
            );
        }

        Ok(Some(payload))
    }

    pub fn receive_all(&mut self) -> Result<Vec<PeerMessage>, String> {
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

    pub fn send(&mut self, payload: &PeerMessage) -> Result<bool, String> {
        let header_size = std::mem::size_of::<PeerMessageHeader>();
        let payload_size = bincode::serialized_size(&payload).unwrap() as usize;
        let total_size = header_size + payload_size;

        let mut buffer = Vec::with_capacity(total_size as usize);
        buffer.resize(total_size, 0);
        bincode::serialize_into(
            &mut buffer[..header_size],
            &PeerMessageHeader {
                payload_size: payload_size as u32,
            },
        )
        .unwrap();
        bincode::serialize_into(&mut buffer[header_size..], &payload).unwrap();

        match self.tcp_stream.write(&buffer[..]) {
            Ok(_) => {
                if self.enable_logging {
                    log_info!(
                        "Send [{}] {}",
                        self.peer_address,
                        serde_json::to_string_pretty(&payload).unwrap()
                    );
                }
                Ok(true)
            }
            Err(e) => match e.kind() {
                ErrorKind::WouldBlock => Ok(false),
                _ => Err(e.to_string()),
            },
        }
    }
}
