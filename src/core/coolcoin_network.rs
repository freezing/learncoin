use crate::core::peer_connection::PeerMessage;
use crate::core::PeerConnection;
use std::io::{Error, ErrorKind, Read};
use std::net::{SocketAddr, TcpListener, TcpStream};

pub struct NetworkParams {
    // Address at which TCP server (which listens for peer connections) runs.
    server_address: String,
    // List of peer addresses to connect to.
    peers: Vec<String>,
}

impl NetworkParams {}

pub struct CoolcoinNetwork {
    peer_connections: Vec<(String, PeerConnection)>,
    tcp_listener: TcpListener,
}

impl CoolcoinNetwork {
    pub fn connect(params: &NetworkParams) -> Result<Self, String> {
        let tcp_listener = TcpListener::bind(&params.server_address).map_err(|e| e.to_string())?;
        tcp_listener
            .set_nonblocking(true)
            .map_err(|e| e.to_string())?;

        let mut peer_connections = Vec::new();
        for address in &params.peers {
            let peer_connection = PeerConnection::connect(address.clone())?;
            peer_connections.push((address.clone(), peer_connection));
        }
        Ok(Self {
            peer_connections,
            tcp_listener,
        })
    }

    pub fn accept_new_peers(&mut self) -> Result<(), String> {
        loop {
            match self.tcp_listener.accept() {
                Ok((tcp_stream, socket_address)) => {
                    self.on_new_peer_connected(socket_address, tcp_stream);
                }
                Err(e) => match e.kind() {
                    ErrorKind::WouldBlock => {
                        break;
                    }
                    _ => {
                        return Err(e.to_string());
                    }
                },
            }
        }
        Ok(())
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

    fn on_new_peer_connected(&mut self, socket_address: SocketAddr, tcp_stream: TcpStream) {
        let peer_connection = PeerConnection::from_tcp_stream(socket_address, tcp_stream);
        self.peer_connections
            .push((peer_connection.address().to_string(), peer_connection));
    }
}
