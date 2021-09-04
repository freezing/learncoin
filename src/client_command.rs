use crate::core::block::BlockHash;
use crate::core::coolcoin_network::NetworkParams;
use crate::core::hash::from_hex;
use crate::core::peer_connection::PeerMessage;
use crate::core::{CoolcoinNetwork, CoolcoinNode, PeerConnection, Sha256};
use clap::{App, Arg, ArgMatches};
use std::error::Error;
use std::time::{Duration, SystemTime};

pub struct ClientCliOptions {
    server: String,
    timeout: Duration,
    enable_logging: bool,
}

impl ClientCliOptions {
    pub fn parse(matches: &ArgMatches) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            server: matches.value_of("server").unwrap().to_string(),
            timeout: matches
                .value_of_t::<u64>("timeout")
                .map(Duration::from_secs)?,
            enable_logging: matches.is_present("enable_logging"),
        })
    }
}

fn getblock_subcommand() -> App<'static> {
    App::new("getblock")
        .about("Retrieves the block from the server.")
        .arg(Arg::new("BLOCK_HASH").required(true).index(1))
}

pub fn client_command() -> App<'static> {
    App::new("client")
        .version("0.1")
        .about("Coolcoin client to communicate with the server.")
        .arg(
            Arg::new("server")
                .short('s')
                .long("server")
                .value_name("HOSTNAME:PORT")
                .about("Address of the server that the client cli talks to.")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("timeout")
                .short('t')
                .long("timeout")
                .value_name("TIMEOUT")
                .about("Time to wait for the response.")
                .takes_value(true)
                .required(false)
                .default_value("5"),
        )
        .arg(
            Arg::new("enable_logging")
                .long("enable_logging")
                .about("If true, the messages sent and received via the network are logged.")
                .takes_value(false)
                .required(false),
        )
        .subcommand(getblock_subcommand())
}

fn send_request(client_options: &ClientCliOptions, message: PeerMessage) -> Result<(), String> {
    let mut connection =
        PeerConnection::connect(client_options.server.clone(), client_options.enable_logging)?;
    connection.send(&message)?;
    let request_sent_time = SystemTime::now();
    while request_sent_time.elapsed().unwrap() < client_options.timeout {
        match connection.receive().unwrap() {
            None => continue,
            Some(PeerMessage::ResponseBlock(block)) => {
                let json = serde_json::to_string_pretty(&block).unwrap();
                println!("{}", json);
                return Ok(());
            }
            Some(unexpected) => {
                let json = serde_json::to_string_pretty(&unexpected).unwrap();
                return Err(format!("Got unexpected message: {}", json));
            }
        }
    }
    Err(format!(
        "Request timed out after: {} seconds.",
        client_options.timeout.as_secs()
    ))
}

pub fn run_client(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let client_options = ClientCliOptions::parse(matches)?;

    if let Some(ref matches) = matches.subcommand_matches("getblock") {
        let hex = matches.value_of("BLOCK_HASH").unwrap();
        let block_hash = BlockHash::new(
            from_hex(&hex).map_err(|e| format!("Invalid block hash format: {}", e))?,
        );
        send_request(&client_options, PeerMessage::GetBlock(block_hash))?;
    } else {
        panic!("Should report help.");
    }

    Ok(())
}
