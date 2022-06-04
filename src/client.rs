use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{Duration, Instant};

use crate::{
    Graphwiz, JsonRpcMethod, JsonRpcRequest, JsonRpcResponse, JsonRpcResult, PeerConnection,
    PeerMessagePayload, VersionMessage,
};
use std::fs;

const MAX_RECV_BUFFER_SIZE: usize = 10_000_000;
const VERSION: u32 = 1;

#[derive(Debug)]
pub enum GetBlockchainFormat {
    Graphwiz,
}

impl FromStr for GetBlockchainFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "graphwiz" => Ok(Self::Graphwiz),
            unknown => Err(format!("Unknown GetBlockchainFormat: {}", unknown)),
        }
    }
}

pub struct Client {
    peer_connection: PeerConnection,
    timeout: Duration,
    next_json_rpc_id: u64,
}

impl Client {
    pub fn connect_with_handshake(server: String, timeout: Duration) -> Result<Self, String> {
        let mut peer_connection = PeerConnection::connect(server, MAX_RECV_BUFFER_SIZE)?;
        let mut client = Self {
            peer_connection,
            timeout,
            next_json_rpc_id: 0,
        };
        client.send_message(&PeerMessagePayload::Version(VersionMessage::new(VERSION)))?;
        match client.wait_for_response()? {
            PeerMessagePayload::Verack => Ok(client),
            unexpected => Err(format!("Received unexpected message: {:?}", unexpected)),
        }
    }

    pub fn execute_get_blockchain(
        &mut self,
        format: GetBlockchainFormat,
        suffix_length: usize,
        output_file: &str,
    ) -> Result<(), String> {
        let id = self.send_json_rpc_request(JsonRpcMethod::GetBlockchain)?;
        match self.wait_for_json_rpc_response(id)? {
            JsonRpcResponse { id, result } => match result? {
                JsonRpcResult::Blockchain(blocks, active_block_hashes) => {
                    let data = match format {
                        GetBlockchainFormat::Graphwiz => {
                            Graphwiz::blockchain(blocks, &active_block_hashes, suffix_length)
                        }
                    };
                    fs::write(output_file, data).map_err(|e| e.to_string())?;
                    Ok(())
                }
                unexpected => Err(format!("Received unexpected message: {:?}", unexpected)),
            },
        }
    }

    fn wait_for_json_rpc_response(&mut self, expected_id: u64) -> Result<JsonRpcResponse, String> {
        match self.wait_for_response()? {
            PeerMessagePayload::JsonRpcResponse(response) if response.id == expected_id => {
                Ok(response)
            }
            unexpected => Err(format!("Received unexpected message: {:?}", unexpected)),
        }
    }

    fn wait_for_response(&mut self) -> Result<PeerMessagePayload, String> {
        let instant = Instant::now();
        while instant.elapsed().lt(&self.timeout) {
            match self.peer_connection.receive()? {
                None => continue,
                Some(message) => return Ok(message),
            }
        }
        Err(format!(
            "Timed out after {}ms while waiting for the handshake to complete.",
            self.timeout.as_millis()
        ))
    }

    fn send_json_rpc_request(&mut self, method: JsonRpcMethod) -> Result<u64, String> {
        let id = self.next_json_rpc_id();
        self.send_message(&PeerMessagePayload::JsonRpcRequest(JsonRpcRequest {
            id,
            method,
        }))?;
        Ok(id)
    }

    fn next_json_rpc_id(&mut self) -> u64 {
        let id = self.next_json_rpc_id;
        self.next_json_rpc_id += 1;
        id
    }

    fn send_message(&mut self, message: &PeerMessagePayload) -> Result<(), String> {
        let is_sent = self.peer_connection.send(&message)?;
        if !is_sent {
            Err(format!(
                "Failed to send the version message due to flow-control."
            ))
        } else {
            Ok(())
        }
    }
}
