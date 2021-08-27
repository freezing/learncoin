use std::error::Error;
use clap::{App, AppSettings};

fn main() -> Result<(), Box<dyn Error>> {
    let matches =
        App::new("coolcoin")
            .about("Coolcoin blockchain CLI apps.")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .subcommand(coolcoin_lib::daemon_command::daemon_command())
            .get_matches();

    if let Some(ref matches) = matches.subcommand_matches("daemon") {
        let options = coolcoin_lib::daemon_command::DaemonCliOptions::parse(*matches)?;
        coolcoin_lib::daemon_command::run_daemon(&options)
    } else {
        panic!("Should report help.");
    }
}
