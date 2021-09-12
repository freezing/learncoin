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

pub fn client_command() -> App<'static> {
    App::new("client")
        .version("0.1")
        .about("Coolcoin client to interact with the server.")
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
}

pub fn run_client_command(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    unimplemented!()
}
