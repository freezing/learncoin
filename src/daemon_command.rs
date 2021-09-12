use crate::core::coolcoin_network::NetworkParams;
use crate::core::{Address, CoolcoinNetwork, CoolcoinNode};
use clap::{App, Arg, ArgMatches};
use std::error::Error;

pub struct DaemonCliOptions {
    server: String,
    peers: Vec<String>,
    enable_logging: bool,
    coinbase_address: Address,
}

impl DaemonCliOptions {
    pub fn parse(matches: &ArgMatches) -> Result<Self, Box<dyn Error>> {
        let peers = matches
            .values_of("peers")
            .map(|v| v.collect())
            .unwrap_or_else(|| vec![])
            .iter()
            .map(|s| s.to_string())
            .collect();
        let enable_logging = matches.is_present("enable_logging");
        let coinbase_address = matches.value_of("coinbase_address").unwrap().to_string();

        Ok(Self {
            server: matches.value_of("server").unwrap().to_string(),
            peers,
            enable_logging,
            coinbase_address: Address::new(coinbase_address),
        })
    }
}

pub fn daemon_command() -> App<'static> {
    App::new("daemon")
        .version("0.1")
        .about("Coolcoin daemon process.")
        .arg(
            Arg::new("server")
                .short('s')
                .long("server")
                .value_name("HOSTNAME:PORT")
                .about("Address at which the daemon runs servers for peers to connect to.")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("peers")
                .long("peers")
                .value_name("[String]")
                .about("List of peer addresses to which the node connects to.")
                .multiple_occurrences(true)
                .use_delimiter(true)
                .takes_value(true)
                .default_values(vec![].as_slice())
                .required(false),
        )
        .arg(
            Arg::new("enable_logging")
                .long("enable_logging")
                .about("If true, the messages sent and received via the network are logged.")
                .takes_value(false)
                .required(false),
        )
        .arg(
            Arg::new("coinbase_address")
                .long("coinbase_address")
                .value_name("ADDRESS")
                .about("Address at which the miner gets paid for successfully mining a block.")
                .takes_value(true)
                .required(true),
        )
}

pub fn run_daemon(options: &DaemonCliOptions) -> Result<(), Box<dyn Error>> {
    println!("Starting full node!");
    let network_params = NetworkParams::new(
        options.server.clone(),
        options.peers.clone(),
        options.enable_logging,
    );
    let mut node = CoolcoinNode::connect(network_params, options.coinbase_address.clone())?;
    node.run();
    Ok(())
}
