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
    // Whether or not the messages that are sent and received through the network are logged.
    enable_logging: bool,
}

impl NetworkParams {
    pub fn new(server_address: String, peer_addresses: Vec<String>, enable_logging: bool) -> Self {
        Self {
            server_address,
            peers: peer_addresses,
            enable_logging,
        }
    }
}

pub struct CoolcoinNetwork {
    peer_connections: Vec<(String, PeerConnection)>,
    enable_logging: bool,
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
            let peer_connection = PeerConnection::connect(address.clone(), params.enable_logging)?;
            peer_connections.push((address.clone(), peer_connection));
        }
        Ok(Self {
            peer_connections,
            tcp_listener,
            send_queue: vec![],
            enable_logging: params.enable_logging,
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
                    eprintln!("{}", e);
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

    pub fn multicast(&mut self, message: PeerMessage, skipped: Vec<String>) -> Result<(), String> {
        let mut errors = vec![];
        let mut to_drop = HashSet::new();
        for (receiver, connection) in &mut self.peer_connections {
            match skipped.contains(receiver) {
                true => {}
                false => match connection.send(&message) {
                    Ok(_) => {}
                    Err(e) => {
                        to_drop.insert(receiver.to_string());
                        errors.push(e);
                    }
                },
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

    pub fn broadcast(&mut self, message: PeerMessage) -> Result<(), String> {
        self.multicast(message, vec![])
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
        let peer_connection =
            PeerConnection::from_tcp_stream(socket_address, tcp_stream, self.enable_logging);
        self.peer_connections
            .push((peer_connection.address().to_string(), peer_connection));
    }

    fn drop_connection(&mut self, sender: &str) {
        for i in 0..self.peer_connections.len() {
            let (peer_address, _) = self.peer_connections.get(i).unwrap();
            if peer_address == sender {
                self.peer_connections.remove(i);
                break;
            }
        }
    }
}
