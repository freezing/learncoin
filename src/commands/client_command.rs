use crate::{Client, GetBlockchainFormat};
use clap::{App, Arg, ArgMatches};
use std::error::Error;
use std::time::Duration;

struct ClientCliOptions {
    server: String,
    timeout: Duration,
}

impl ClientCliOptions {
    pub fn parse(matches: &ArgMatches) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            server: matches.value_of("server").unwrap().to_string(),
            timeout: matches
                .value_of_t::<u64>("timeout")
                .map(Duration::from_secs)?,
        })
    }
}

fn get_blockchain() -> App<'static> {
    App::new("get-blockchain")
        .version("0.1")
        .about("Requests the full blockchain from the local node and prints it to the output file.")
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .about("Output format of the printed blockchain.")
                .takes_value(true)
                .default_value("graphwiz")
                .required(false),
        )
        .arg(
            Arg::new("suffix-length")
                .long("suffix-length")
                .about("Length of the block hash suffix that is printed.")
                .takes_value(true)
                .default_value("8")
                .required(false),
        )
        .arg(
            Arg::new("output-file")
                .long("output-file")
                .about("File to which the output is printed.")
                .takes_value(true)
                .required(true),
        )
}

pub fn client_command() -> App<'static> {
    App::new("client")
        .version("0.1")
        .about("LearnCoin client to interact with the server.")
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
        .subcommand(get_blockchain())
}

pub fn run_client_command(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let options = ClientCliOptions::parse(matches)?;
    let mut client = Client::connect_with_handshake(options.server, options.timeout)?;

    if let Some(ref matches) = matches.subcommand_matches("get-blockchain") {
        let format = matches.value_of_t("format")?;
        let hash_suffix = matches.value_of_t("suffix-length")?;
        let output_file = matches.value_of("output-file").unwrap();
        client.execute_get_blockchain(format, hash_suffix, output_file)?;
        Ok(())
    } else {
        panic!("No command has been specified")
    }
}
