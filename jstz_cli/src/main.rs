use clap::{Parser, Subcommand};

mod deposit;
mod deploy_bridge;
mod deploy_contract;
mod run_contract;
mod sandbox;
//mod repl;
mod config; 
mod sandbox_initializer;
mod utils;

use deposit::deposit;
//use crate::deploy_bridge::deploy_bridge;
use deploy_contract::deploy_contract;
use run_contract::run_contract;
use sandbox::sandbox_stop;
use std::env;
//use crate::sandbox::repl;
use config::Config;

use std::process::Command;
use std::fs::File;

#[derive(Parser)]
#[command(author, version)]
struct JstzCli {
    /// Path to the jstz configuration file.
    #[arg(short, long, value_name = "CONFIG_FILE", default_value = "jstz.json")]
    config_file: String,

    #[command(subcommand)]
    command: JstzCommand,
}

#[derive(Subcommand)]
enum JstzCommand {
    /// Commands related to the jstz sandbox.
    #[command(subcommand)]
    Sandbox(SandboxCommand),
    /// Deposits from an existing L1 sandbox address to a L2 sandbox address.
    #[command(name = "bridge-deposit")]
    BridgeDeposit {
        /// The L1 sandbox address or alias to withdraw from.
        #[arg(short, long)]
        from: String,
        /// The L2 sandbox address or alias to deposit to.
        #[arg(short, long)]
        to: String,
        /// The amount in ctez to transfer.
        #[arg(short, long)]
        amount: u64,
    },
    /*
    /// Publishes the given script to the local sandbox.
    DeployBridge {
        /// Path to the contract script.
        #[arg(value_name = "SCRIPT_PATH")]
        script: String,
        /// Alias for the address of the deployed contract.
        #[arg(short, long)]
        name: String,//Option<String>,
    },
    */
    DeployContract {
        /// Contract address when executing the contract.
        #[arg(short, long)]
        self_address: String, 
        /// Contract code.
        #[arg(short, long)]
        contract_code: String, 
        /// Initial balance
        #[arg(short, long)]
        balance: u64
    },
    /// Run a contract using a specified URL.
    Run {
        /// The URL containing the contract's address or alias.
        #[arg(value_name = "URL")]
        url: String,
        /// The code of the contract.
        #[arg(value_name = "referrer")]
        referrer: String,
        /// The HTTP method used in the request.
        #[arg(name = "request", short, long, default_value = "GET")]
        http_method: String,
        /// The JSON data in the request body.
        #[arg(name = "data", short, long, default_value = None)]
        json_data: Option<String>,
    },
    /// Start a REPL session.
    Repl {
        /// Sets the address of the REPL environment.
        #[arg(short, long)]
        self_address: Option<String>,
    },
    ViewConsole,
}

#[derive(Subcommand)]
enum SandboxCommand {
    /// Starts a jstz sandbox, starting an octez-node, rollup node, baker, and deploys the jstz rollup kernel and jstz bridge.
    Start,
    /// Stops the currently running jstz sandbox.
    Stop,
}

fn main() {
    match env::current_dir() {
        Ok(path) => println!("The current directory is {}", path.display()),
        Err(e) => eprintln!("Failed to get current directory: {}", e),
    }

    let cli = JstzCli::parse();

    let mut cfg = match Config::load_from_file() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load the config file: {}", e);
            return;
        }
    };

    match cli.command {
        JstzCommand::Sandbox(cmd) => match cmd {
            SandboxCommand::Start => {
                println!("Starting the jstz sandbox...");
                File::create("sandbox_config.json").expect("Failed to create a file.");
                Command::new("../target/debug/sandbox_process")
                    .spawn().expect("Failed to start the sandbox.");
            }
            SandboxCommand::Stop => {
                println!("Stopping the jstz sandbox...");
                sandbox_stop(&mut cfg);
            }
        },
        JstzCommand::BridgeDeposit {
            mut from,
            mut to,
            amount,
        } => {
            println!("Depositing {} Tez from {} to {}", amount, from, to);

            if let Some(alias) = cfg.get_tz4_alias(&from) {
                println!("Using alias for {}: {}", from, alias);
                from = alias;
            }
            if let Some(alias) = cfg.get_tz4_alias(&to) {
                println!("Using alias for {}: {}", to, alias);
                to = alias;
            }

            deposit(from, to, amount, &cfg);
        },
        /*JstzCommand::DeployBridge { mut script, name } => {
            println!("Deploying script {} with alias {}", script, name);

            if let Some(alias) = cfg.get_name_alias(&name) {
                println!("Using alias for {} instead of script: {}", name, alias);
                script = alias;
            }
            deploy_bridge(script, &cfg);
        },*/
        JstzCommand::DeployContract { mut self_address, contract_code, balance} => {
            if let Some(alias) = cfg.get_name_alias(&self_address) {
                println!("Using alias for {} instead of self_address: {}", self_address, alias);
                self_address = alias;
            }
            deploy_contract(self_address, contract_code, balance, &cfg);
        },
        JstzCommand::Run {
            url,
            referrer,
            http_method,
            json_data,
        } => {
            // println!("Running {} with code {}", url, contract_code);
            // if let Some(alias) = cfg.get_url_alias(&url) {
            //     println!("Using alias for {}: {}", url, alias);
            //     url = alias;
            // }
            run_contract(referrer, url, http_method, json_data, &cfg);
        }
        JstzCommand::Repl { self_address } => {
            if let Some(address) = self_address {
                println!("Starting REPL with self address: {}", address);
                //repl(address)
            } else {
                println!("Starting REPL without a self address");
                //repl()
            }
        },
        JstzCommand::ViewConsole => {}
    }

    if let Err(e) = cfg.save_to_file() {
        eprintln!("Failed to save the config file: {}", e);
    }
}