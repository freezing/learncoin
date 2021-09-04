use clap::{App, AppSettings};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("coolcoin")
        .about("Coolcoin blockchain CLI apps.")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(coolcoin_lib::daemon_command::daemon_command())
        .subcommand(coolcoin_lib::client_command::client_command())
        .get_matches();

    if let Some(ref matches) = matches.subcommand_matches("daemon") {
        let options = coolcoin_lib::daemon_command::DaemonCliOptions::parse(*matches)?;
        coolcoin_lib::daemon_command::run_daemon(&options)
    } else if let Some(ref matches) = matches.subcommand_matches("client") {
        coolcoin_lib::client_command::run_client(*matches)
    } else {
        panic!("Should report help.");
    }
}
