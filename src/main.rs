use clap::{App, AppSettings};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("learncoin")
        .about("LearnCoin blockchain CLI tools.")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(learncoin_lib::commands::server_command())
        .subcommand(learncoin_lib::commands::client_command())
        .subcommand(learncoin_lib::commands::miner_command())
        .get_matches();

    if let Some(ref matches) = matches.subcommand_matches("server") {
        learncoin_lib::commands::run_server_command(&matches)
    } else if let Some(ref matches) = matches.subcommand_matches("client") {
        learncoin_lib::commands::run_client_command(&matches)
    } else if let Some(ref matches) = matches.subcommand_matches("miner") {
        learncoin_lib::commands::run_miner_command(&matches)
    } else {
        panic!("Should report help.");
    }
}
