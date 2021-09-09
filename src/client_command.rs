use crate::core::block::BlockHash;
use crate::core::coolcoin_network::NetworkParams;
use crate::core::hash::from_hex;
use crate::core::peer_connection::PeerMessage;
use crate::core::transaction::{OutputIndex, TransactionId, TransactionInput, TransactionOutput};
use crate::core::{
    as_hex, Address, Block, BlockchainManager, Coolcoin, CoolcoinNetwork, CoolcoinNode,
    PeerConnection, Sha256, Transaction,
};
use clap::{App, Arg, ArgMatches};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct ClientCliOptions {
    server: String,
    timeout: Duration,
    enable_logging: bool,
}

impl ClientCliOptions {
    pub fn parse(matches: &ArgMatches) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            server: matches.value_of("server").unwrap().to_string(),
            timeout: matches
                .value_of_t::<u64>("timeout")
                .map(Duration::from_secs)?,
            enable_logging: matches.is_present("enable_logging"),
        })
    }
}

fn getfullblockchain_subcommand() -> App<'static> {
    App::new("getfullblockchain")
        .about("Retrieves the full block from the server (including non-active chains).")
}

fn getblock_subcommand() -> App<'static> {
    App::new("getblock")
        .about("Retrieves the block from the server.")
        .arg(Arg::new("BLOCK_HASH").required(true).index(1))
}

fn sendrawtransaction_subcommand() -> App<'static> {
    App::new("sendrawtransaction")
        .about("Sends the given raw transaction to the server.")
        .arg(Arg::new("inputs")
            .long("inputs")
            .about("The list of inputs as references to the unspent outputs. Format: <TXID>:<OutputIndex> ")
            .multiple_occurrences(true)
            .takes_value(true)
            .use_delimiter(true)
            .required(true))
        .arg(Arg::new("outputs")
            .long("outputs")
            .use_delimiter(true)
            .about("The list of outputs and amounts. Format: <CoolcoinAddress>:<Amount> ")
            .multiple_occurrences(true)
            .takes_value(true)
            .required(true))
}

pub fn client_command() -> App<'static> {
    App::new("client")
        .version("0.1")
        .about("Coolcoin client to communicate with the server.")
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
        .arg(
            Arg::new("enable_logging")
                .long("enable_logging")
                .about("If true, the messages sent and received via the network are logged.")
                .takes_value(false)
                .required(false),
        )
        .subcommand(getfullblockchain_subcommand())
        .subcommand(getblock_subcommand())
        .subcommand(sendrawtransaction_subcommand())
}

fn short_hash(hash: &BlockHash, blocks: &HashMap<&BlockHash, &Block>) -> String {
    // TODO: This is a hack for now.
    (&as_hex(&hash.as_slice())[..8]).to_string()
}

fn graphviz(blocks: &Vec<Block>) -> Result<(), String> {
    // TODO: Hihglight active blockchain and orphans.
    // digraph G {
    //
    //   subgraph cluster_0 {
    //     style=filled;
    //     color=lightgrey;
    //     node [style=filled,color=white];
    //     a0 -> a1 -> a2 -> a3;
    //     label = "Active";
    //   }
    //
    //   a0 -> b1 -> b2 -> b3
    //
    //   subgraph cluster_1 {
    //     style=filled;
    //     color=lightgrey;
    //     node [style=filled,color=white];
    //     c0;
    //     c1;
    //     c2;
    //     c3;
    //     c4;
    //     ce91c6 -> 9ce91c7
    //     label = "Orphans";
    //   }
    // }
    let blocks = blocks
        .iter()
        .map(|b| (b.id(), b))
        .collect::<HashMap<&BlockHash, &Block>>();

    let graph_contents = blocks
        .iter()
        .map(|(hash, block)| {
            (
                blocks
                    .get(block.header().previous_block_hash())
                    .map(|b| b.id()),
                block.id(),
            )
        })
        .map(|(parent, child)| match parent {
            Some(parent) => format!(
                r#""{}" -> "{}";"#,
                short_hash(parent, &blocks),
                short_hash(child, &blocks)
            ),
            None => format!(r#""{}";"#, short_hash(child, &blocks)),
        })
        .collect::<Vec<String>>()
        .join("\n");

    let contents = format!(
        r#"
    digraph G {{
      {}
    }}
    "#,
        graph_contents
    );
    fs::write("./blockchain.dot", contents).map_err(|e| e.to_string())
}

fn send_request(client_options: &ClientCliOptions, message: PeerMessage) -> Result<(), String> {
    let mut connection =
        PeerConnection::connect(client_options.server.clone(), client_options.enable_logging)?;
    connection.send(&message)?;
    let request_sent_time = SystemTime::now();
    while request_sent_time.elapsed().unwrap() < client_options.timeout {
        match connection.receive().unwrap() {
            None => continue,
            Some(PeerMessage::ResponseBlock(block)) => {
                let json = serde_json::to_string_pretty(&block).unwrap();
                println!("{}", json);
                return Ok(());
            }
            Some(PeerMessage::ResponseTransaction) => {
                println!("Success");
                return Ok(());
            }
            Some(PeerMessage::ResponseFullBlockchain(blocks)) => {
                graphviz(&blocks)?;

                // tODO: Split ohrpnaed and active
                let json = serde_json::to_string_pretty(&blocks).unwrap();
                println!("{}", json);
                let mut blockchain_manager = BlockchainManager::new();
                for block in blocks {
                    blockchain_manager.new_block_reinsert_orphans(block);
                }

                println!("Active blockchain");
                let mut width = 0 as usize;
                for block in blockchain_manager.block_tree().active_blockchain() {
                    println!("{}{}", " ".repeat(width), block.id());
                    width += 4;
                }
                return Ok(());
            }
            Some(unexpected) => {
                let json = serde_json::to_string_pretty(&unexpected).unwrap();
                return Err(format!("Unexpected:{}", json));
            }
        }
    }
    Err(format!(
        "Request timed out after: {} seconds.",
        client_options.timeout.as_secs()
    ))
}

pub fn run_client(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let client_options = ClientCliOptions::parse(matches)?;

    if let Some(ref matches) = matches.subcommand_matches("getblock") {
        let hex = matches.value_of("BLOCK_HASH").unwrap();
        let block_hash = BlockHash::new(
            from_hex(&hex).map_err(|e| format!("Invalid block hash format: {}", e))?,
        );
        send_request(&client_options, PeerMessage::GetBlock(block_hash))?;
    } else if let Some(ref matches) = matches.subcommand_matches("sendrawtransaction") {
        let locktime = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as u32;
        let inputs = matches
            .values_of("inputs")
            .unwrap()
            .map(|input| {
                let mut tokens = input.split(":");
                let txid = TransactionId::new(from_hex(tokens.next().unwrap()).unwrap());
                let output_index = OutputIndex::new(tokens.next().unwrap().parse::<i32>().unwrap());
                TransactionInput::new(txid, output_index)
            })
            .collect::<Vec<TransactionInput>>();
        let outputs = matches
            .values_of("outputs")
            .unwrap()
            .map(|output| {
                let mut tokens = output.split(":").collect::<Vec<&str>>();
                println!("Tokens: {:?}", tokens);
                let address = Address::new(tokens.get(0).unwrap().to_string());
                let amount = Coolcoin::new(tokens.get(1).unwrap().parse::<i64>().unwrap());
                TransactionOutput::new(address, amount)
            })
            .collect::<Vec<TransactionOutput>>();
        let transaction = Transaction::new(inputs, outputs, locktime)?;
        send_request(&client_options, PeerMessage::SendTransaction(transaction))?;
    } else if let Some(ref matchesa) = matches.subcommand_matches("getfullblockchain") {
        send_request(&client_options, PeerMessage::GetFullBlockchain)?;
    } else {
        panic!("Should report help.");
    }

    Ok(())
}
