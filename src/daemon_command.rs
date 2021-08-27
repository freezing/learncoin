use clap::{App, ArgMatches};
use std::error::Error;

pub struct DaemonCliOptions {}

impl DaemonCliOptions {
    pub fn parse(_matches: &ArgMatches) -> Result<Self, Box<dyn Error>> {
        Ok(Self {})
    }
}

pub fn daemon_command() -> App<'static> {
    App::new("daemon")
        .version("0.1")
        .about("Coolcoin daemon process.")
}

pub fn run_daemon(_options: &DaemonCliOptions) -> Result<(), Box<dyn Error>> {
    println!("Running daemon process!");
    Ok(())
}
