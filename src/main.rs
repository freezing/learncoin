use clap::{App, AppSettings};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("coolcoin")
        .about("Coolcoin blockchain CLI tools.")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(coolcoin_lib::commands::server_command())
        .subcommand(coolcoin_lib::commands::client_command())
        .get_matches();

    if let Some(ref matches) = matches.subcommand_matches("server") {
        coolcoin_lib::commands::run_server_command(&matches)
    } else if let Some(ref matches) = matches.subcommand_matches("client") {
        coolcoin_lib::commands::run_client_command(&matches)
    } else {
        panic!("Should report help.");
    }
}
