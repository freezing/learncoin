use crate::core::peer_connection::PeerMessage;
use crate::core::PeerConnection;
use std::collections::HashSet;
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
    send_queue: Vec<(String, PeerMessage)>,
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
            send_queue: vec![],
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

    pub fn receive_all(&mut self) -> Vec<(String, PeerMessage)> {
        let mut all_messages = vec![];
        let mut to_drop = HashSet::new();
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
                    to_drop.insert(sender.clone());
                    continue;
                }
            }
        }

        for peer_address in to_drop {
            self.drop_connection(&peer_address);
        }

        all_messages
    }

    pub fn broadcast(&mut self, message: PeerMessage) -> Result<(), String> {
        let mut errors = vec![];
        let mut to_drop = HashSet::new();
        for (receiver, connection) in &mut self.peer_connections {
            match connection.send(&message) {
                Ok(_) => {}
                Err(e) => {
                    to_drop.insert(receiver.to_string());
                    errors.push(e);
                }
            }
        }

        for peer_address in to_drop {
            self.drop_connection(&peer_address);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n"))
        }
    }

    pub fn send_to(&mut self, receiver: &str, message: PeerMessage) -> Result<bool, String> {
        match self
            .peer_connections
            .iter_mut()
            .find(|(address, _)| address == receiver)
        {
            None => Err(format!("Peer: {} doesn't exist.", receiver)),
            Some((_, peer)) => peer.send(&message),
        }
    }

    fn on_new_peer_connected(&mut self, socket_address: SocketAddr, tcp_stream: TcpStream) {
        let peer_connection = PeerConnection::from_tcp_stream(socket_address, tcp_stream);
        self.peer_connections
            .push((peer_connection.address().to_string(), peer_connection));
    }

    fn drop_connection(&mut self, sender: &str) {
        // TODO: Drop connection and try to find another one.
    }
}
