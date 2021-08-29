use crate::core::peer_connection::PeerMessage;
use crate::core::PeerConnection;
use std::io::{Error, Read};
use std::net::TcpStream;

pub struct NetworkParams {
    // List of peer addresses to connect to.
    peers: Vec<String>,
}

impl NetworkParams {
    pub fn default() -> Self {
        Self { peers: vec![] }
    }
}

pub struct CoolcoinNetwork {
    peer_connections: Vec<(String, PeerConnection)>,
}

impl CoolcoinNetwork {
    pub fn connect(params: &NetworkParams) -> Result<Self, String> {
        let mut peer_connections = Vec::new();
        for address in &params.peers {
            let peer_connection = PeerConnection::connect(address.clone())?;
            peer_connections.push((address.clone(), peer_connection));
        }
        Ok(Self { peer_connections })
    }

    pub fn receive_data(&mut self) -> Vec<(String, PeerMessage)> {
        let mut all_messages = vec![];
        for (sender, peer_connection) in &mut self.peer_connections {
            match peer_connection.receive_all() {
                Ok(messages) => {
                    for message in messages {
                        all_messages.push((sender.clone(), message));
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Got an error while reading from peer: {}. Error: {}",
                        sender, e
                    );
                    // TODO: Drop the connection and try to find another node.
                    continue;
                }
            }
        }
        all_messages
    }
}
