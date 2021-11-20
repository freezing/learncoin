use crate::miner::{Miner, MinerParams};
use clap::{App, Arg, ArgMatches};
use std::error::Error;
use std::time::Duration;

const MAX_RECV_BUFFER_SIZE: usize = 10_000;

struct MinerCliOptions {
    server: String,
}

impl MinerCliOptions {
    pub fn parse(matches: &ArgMatches) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            server: matches.value_of("server").unwrap().to_string(),
        })
    }
}

pub fn miner_command() -> App<'static> {
    App::new("miner")
        .version("0.1")
        .about("LearnCoin miner that searches for the PoW solution.")
        .arg(
            Arg::new("server")
                .short('s')
                .long("server")
                .value_name("HOSTNAME:PORT")
                .about("Address of the server that the miner connects to.")
                .takes_value(true)
                .required(true),
        )
}

pub fn run_miner_command(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let options = MinerCliOptions::parse(matches)?;
    let miner = Miner::new(MinerParams {
        server_address: options.server,
        recv_buffer_size: MAX_RECV_BUFFER_SIZE,
    })?;
    miner.run()?;
    Ok(())
}
