use crate::{
    Client, GetBlockchainFormat, LockingScript, OutputIndex, PublicKey, Sha256, Transaction,
    TransactionId, TransactionInput, TransactionOutput,
};
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

fn get_balances() -> App<'static> {
    App::new("get-balances")
        .version("0.1")
        .about("Retrieves balances for each public address on the blockchain.")
}

fn send_transaction() -> App<'static> {
    App::new("send-transaction")
        .version("0.1")
        .about(
            "Send a transaction with a single transaction input and multiple transaction outputs.",
        )
        .arg(
            Arg::new("input")
                .long("input")
                .value_name("TXID:INDEX")
                .about("Unspent transaction output formatted as <txid:output_index>")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("outputs")
                .long("outputs")
                .value_name("Comma-separated list of <PublicKey>:<Amount>")
                .takes_value(true)
                .required(true)
                .multiple_values(true)
                .use_delimiter(true),
        )
}

fn get_transaction_outputs() -> App<'static> {
    App::new("get-transaction-outputs")
        .version("0.1")
        .about("Retrieves transaction outputs.")
        .arg(
            Arg::new("utxo-only")
                .long("utxo-only")
                .about("If set, prints only unspent transaction outputs.")
                .takes_value(false)
                .required(false),
        )
}

fn get_transaction() -> App<'static> {
    App::new("get-transaction")
        .version("0.1")
        .about("Retrieves information about a single transaction.")
        .arg(
            Arg::new("id")
                .long("id")
                .about("ID of the transaction.")
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
        .subcommand(get_balances())
        .subcommand(send_transaction())
        .subcommand(get_transaction_outputs())
        .subcommand(get_transaction())
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
    } else if let Some(ref matches) = matches.subcommand_matches("get-balances") {
        client.execute_get_balances()?;
        Ok(())
    } else if let Some(ref matches) = matches.subcommand_matches("send-transaction") {
        let transaction_input = matches.value_of("input").unwrap();
        let mut tokens = transaction_input.split(":");
        let utxo_id = TransactionId::new(
            Sha256::from_hex(tokens.next().expect("input format must be <txid:index>")).unwrap(),
        );
        let output_index = OutputIndex::new(
            tokens
                .next()
                .expect("input format must be <txid:index>")
                .parse::<i32>()
                .unwrap(),
        );
        let transaction_input = TransactionInput::new(utxo_id, output_index);
        let transaction_outputs: Vec<TransactionOutput> = matches
            .values_of_lossy("outputs")
            .unwrap()
            .into_iter()
            .map(|balances| {
                let mut tokens = balances.split(":");
                let locking_script = LockingScript::new(PublicKey::new(
                    tokens
                        .next()
                        .expect("output format must be list of <pubkey:amount>")
                        .to_owned(),
                ));
                let amount = tokens
                    .next()
                    .expect("output format must be list of <pubkey:amount>")
                    .parse::<i64>()
                    .unwrap();
                TransactionOutput::new(amount, locking_script)
            })
            .collect();
        client.execute_send_transaction(transaction_input, transaction_outputs)?;
        Ok(())
    } else if let Some(ref matches) = matches.subcommand_matches("get-transaction-outputs") {
        let utxo_only = matches.is_present("utxo-only");
        client.execute_get_transaction_outputs(utxo_only)?;
        Ok(())
    } else if let Some(ref matches) = matches.subcommand_matches("get-transaction") {
        let id = TransactionId::new(Sha256::from_hex(matches.value_of("id").unwrap()).unwrap());
        client.execute_get_transaction(id)?;
        Ok(())
    } else {
        panic!("No command has been specified")
    }
}
