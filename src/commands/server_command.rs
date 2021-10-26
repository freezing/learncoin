use crate::{LearnCoinNode, NetworkParams};
use clap::{App, Arg, ArgMatches};
use std::error::Error;

const MAX_RECV_BUFFER_SIZE: usize = 10_000;
const SOFTWARE_VERSION: u32 = 1;

struct ServerCliOptions {
    address: String,
    peers: Vec<String>,
}

impl ServerCliOptions {
    pub fn parse(matches: &ArgMatches) -> Result<Self, Box<dyn Error>> {
        let peers = matches
            .values_of("peers")
            .map(|v| v.collect())
            .unwrap_or_else(|| vec![])
            .iter()
            .map(|s| s.to_string())
            .collect();

        Ok(Self {
            address: matches.value_of("address").unwrap().to_string(),
            peers,
        })
    }
}

pub fn server_command() -> App<'static> {
    App::new("server")
        .version("0.1")
        .about("LearnCoin server process.")
        .arg(
            Arg::new("address")
                .long("address")
                .value_name("HOSTNAME:PORT")
                .about("Address at which the server runs.")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("peers")
                .long("peers")
                .value_name("[HOSTNAME:PORT...]")
                .about("List of peers to which the node connects to.")
                .multiple_occurrences(true)
                .use_delimiter(true)
                .takes_value(true)
                .default_values(vec![].as_slice())
                .required(false),
        )
}

pub fn run_server_command(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let options = ServerCliOptions::parse(matches)?;
    let network_params = NetworkParams::new(
        options.address.clone(),
        options.peers.clone(),
        MAX_RECV_BUFFER_SIZE,
    );
    let node = LearnCoinNode::connect(network_params, SOFTWARE_VERSION)?;
    node.run()?;
    Ok(())
}
