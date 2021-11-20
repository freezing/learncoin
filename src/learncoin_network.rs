use std::collections::HashSet;
use std::io::ErrorKind;
use std::net::{SocketAddr, TcpListener, TcpStream};

use crate::{PeerConnection, PeerMessagePayload};

pub struct NetworkParams {
    // Address at which TCP server (which listens for peer connections) runs.
    server_address: String,
    // Initial list of peer addresses to connect to.
    peers: Vec<String>,
    // Size of the receive buffer for each peer connection.
    recv_buffer_size: usize,
}

impl NetworkParams {
    pub fn new(
        server_address: String,
        peer_addresses: Vec<String>,
        recv_buffer_size: usize,
    ) -> Self {
        Self {
            server_address,
            peers: peer_addresses,
            recv_buffer_size,
        }
    }
}

pub struct LearnCoinNetwork {
    params: NetworkParams,
    // A list of all peer connections known to this node.
    peer_connections: Vec<PeerConnection>,
    tcp_listener: TcpListener,
    misbehaving_peers: HashSet<String>,
}

impl LearnCoinNetwork {
    /// Connects to the LearnCoin network.
    /// This function starts a TCP server that listens to new incoming peer connections,
    /// and it connects to the peers specified in the network params.
    pub fn connect(params: NetworkParams) -> Result<Self, String> {
        let tcp_listener = TcpListener::bind(&params.server_address).map_err(|e| e.to_string())?;
        tcp_listener
            .set_nonblocking(true)
            .map_err(|e| e.to_string())?;

        let mut peer_connections = Vec::new();
        for address in &params.peers {
            let peer_connection =
                PeerConnection::connect(address.clone(), params.recv_buffer_size)?;
            peer_connections.push(peer_connection);
        }
        Ok(Self {
            params,
            peer_connections,
            tcp_listener,
            misbehaving_peers: HashSet::new(),
        })
    }

    /// Returns the list of all peer addresses in the network.
    pub fn peer_addresses(&self) -> Vec<&str> {
        self.peer_connections
            .iter()
            .map(PeerConnection::peer_address)
            .collect()
    }

    /// Accepts new incoming peer connections and adds them to the network.
    pub fn accept_new_peers(&mut self) -> Result<Vec<String>, String> {
        let mut new_peers = vec![];
        loop {
            match self.tcp_listener.accept() {
                Ok((tcp_stream, socket_address)) => {
                    new_peers.push(socket_address.to_string());
                    self.on_new_peer_connected(socket_address, tcp_stream);
                }
                Err(e) => match e.kind() {
                    ErrorKind::WouldBlock => {
                        // No new peers are awaiting.
                        break;
                    }
                    _ => {
                        return Err(e.to_string());
                    }
                },
            }
        }
        Ok(new_peers)
    }

    /// Receives all payloads from the network.
    pub fn receive_all(&mut self) -> Vec<(String, Vec<PeerMessagePayload>)> {
        let Self {
            peer_connections,
            misbehaving_peers,
            ..
        } = self;

        let mut all_messages = vec![];
        for peer_connection in peer_connections {
            all_messages.push((
                peer_connection.peer_address().to_string(),
                Self::receive_all_from_peer(misbehaving_peers, peer_connection),
            ));
        }
        all_messages
    }

    /// Sends the payload to the peer.
    /// If send fails or the flow-control pushes back, mark the peer as misbehaving.
    pub fn send(&mut self, peer_address: &str, payload: &PeerMessagePayload) {
        let Self {
            peer_connections,
            misbehaving_peers,
            ..
        } = self;
        for connection in peer_connections {
            if connection.peer_address() == peer_address {
                Self::send_to_peer_connection(connection, &payload, misbehaving_peers);
                return;
            }
        }
        panic!("Called send for unknown peer: {}", peer_address);
    }

    /// Sends the payload to all peers.
    /// See docs for `Self::send`.
    pub fn send_to_all(&mut self, payload: &PeerMessagePayload) {
        let Self {
            peer_connections,
            misbehaving_peers,
            ..
        } = self;
        for connection in peer_connections {
            Self::send_to_peer_connection(connection, &payload, misbehaving_peers);
        }
    }

    /// Forgets about all the peers that caused an error while reading or writing data.
    pub fn drop_misbehaving_peers(&mut self) {
        let Self {
            peer_connections,
            misbehaving_peers,
            ..
        } = self;
        for peer_address in misbehaving_peers.iter() {
            Self::drop_connection(peer_connections, peer_address);
        }
        self.misbehaving_peers.clear();
    }

    pub fn close_peer_connection(&mut self, peer_address: &str) {
        Self::drop_connection(&mut self.peer_connections, peer_address)
    }

    fn on_new_peer_connected(&mut self, socket_address: SocketAddr, tcp_stream: TcpStream) {
        let peer_connection = PeerConnection::from_established_tcp(
            socket_address,
            tcp_stream,
            self.params.recv_buffer_size,
        );
        self.peer_connections.push(peer_connection);
    }

    /// Receives all the messages from the peer connection.
    /// If the read fails, the peer connection is scheduled to be dropped next time
    /// `drop_misbehaving_peers` is called.
    fn receive_all_from_peer(
        misbehaving_peers: &mut HashSet<String>,
        peer_connection: &mut PeerConnection,
    ) -> Vec<PeerMessagePayload> {
        match peer_connection.receive_all() {
            Ok(messages) => messages,
            Err(e) => {
                eprintln!("{}", e);
                misbehaving_peers.insert(peer_connection.peer_address().to_string());
                vec![]
            }
        }
    }

    /// Sends the payload to the given peer connection.
    ///
    /// The payload may not be sent due to the flow-control.
    /// If there is an error while writing to the peer or the peer's receive buffer is full,
    /// i.e. the flow control pushes back, the peer connection is marked as misbehaving.
    /// It is dropped next time `drop_misbehaving_peers` is called.
    fn send_to_peer_connection(
        peer_connection: &mut PeerConnection,
        payload: &PeerMessagePayload,
        misbehaving_peers: &mut HashSet<String>,
    ) {
        match peer_connection.send(payload) {
            Ok(true) => (),
            Ok(false) => {
                misbehaving_peers.insert(peer_connection.peer_address().to_string());
                eprintln!(
                    "Flow-control backoff while sending a message to: {}",
                    peer_connection.peer_address()
                );
            }
            Err(error) => {
                misbehaving_peers.insert(peer_connection.peer_address().to_string());
                eprintln!(
                    "Error while trying to send payload: {:#?}. Reason: {}",
                    payload,
                    error.to_string()
                );
            }
        }
    }

    fn drop_connection(peer_connections: &mut Vec<PeerConnection>, dropped_peer_address: &str) {
        for i in 0..peer_connections.len() {
            let peer_connection = peer_connections.get(i).unwrap();
            if peer_connection.peer_address() == dropped_peer_address {
                // It is sufficient to remove the connection because Rust automatically closes
                // the TCP connection when the object is destroyed.
                peer_connections.remove(i);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_receive() {
        let mut network = LearnCoinNetwork::connect(NetworkParams::new(
            "127.0.0.1:8334".to_string(),
            vec![],
            10_000,
        ))
        .unwrap();
        let mut connection_b =
            PeerConnection::connect("127.0.0.1:8334".to_string(), 10_000).unwrap();
        let connected_peers = network.accept_new_peers().unwrap();
        assert_eq!(connected_peers.len(), 1);

        let payload = PeerMessagePayload::PlaceholderUntilWeImplementProtocol;
        let is_sent = connection_b.send(&payload).unwrap();
        assert!(is_sent);

        let received_messages = network.receive_all();
        assert_eq!(received_messages.len(), 1);
        let (_, received_payloads) = received_messages.first().unwrap();
        assert_eq!(*received_payloads, vec![payload]);
    }
}
