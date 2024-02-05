use std::{
    env,
    fs::{self, File},
    path::{Path, PathBuf},
    process::Child,
    thread::sleep,
    time::Duration,
};

use anyhow::Result;
use jstz_rollup::{
    deploy_ctez_contract, rollup::make_installer, BootstrapAccount, BridgeContract,
    JstzRollup,
};
use log::info;
use octez::OctezThread;
use tempfile::TempDir;
use tokio::task;

use crate::{
    config::{
        Config, SandboxConfig, SANDBOX_OCTEZ_NODE_PORT, SANDBOX_OCTEZ_NODE_RPC_PORT,
        SANDBOX_OCTEZ_SMART_ROLLUP_PORT,
    },
    error::bail_user_error,
};

const SANDBOX_JSTZ_NODE_ADDR: &str = "127.0.0.1";
const SANDBOX_JSTZ_NODE_PORT: u16 = 8933;
const SANDBOX_OCTEZ_SMART_ROLLUP_ADDR: &str = "127.0.0.1";

include!(concat!(env!("OUT_DIR"), "/sandbox_paths.rs"));

fn logs_dir() -> Result<PathBuf> {
    Ok(env::current_dir()?.join("logs"))
}

fn node_log_path() -> Result<PathBuf> {
    Ok(logs_dir()?.join("node.log"))
}

fn client_log_path() -> Result<PathBuf> {
    Ok(logs_dir()?.join("client.log"))
}

const ACTIVATOR_ACCOUNT_SK: &str =
    "unencrypted:edsk31vznjHSSpGExDMHYASz45VZqXN4DPxvsa4hAyY8dHM28cZzp6";

const BOOTSTRAP_ACCOUNT_SKS: [&str; 5] = [
    "unencrypted:edsk3gUfUPyBSfrS9CCgmCiQsTCHGkviBDusMxDJstFtojtc1zcpsh", // bootstrap1
    "unencrypted:edsk39qAm1fiMjgmPkw1EgQYkMzkJezLNewd7PLNHTkr6w9XA2zdfo", // bootstrap2
    "unencrypted:edsk4ArLQgBTLWG5FJmnGnT689VKoqhXwmDPBuGx3z4cvwU9MmrPZZ", // bootstrap3
    "unencrypted:edsk2uqQB9AY4FvioK2YMdfmyMrer5R8mGFyuaLLFfSRo8EoyNdht3", // bootstrap4
    "unencrypted:edsk4QLrcijEffxV31gGdN2HU7UpyJjA8drFoNcmnB28n89YjPNRFm", // bootstrap5
];

const OPERATOR_ADDRESS: &str = "tz1KqTpEZ7Yob7QbPE4Hy4Wo8fHG8LhKxZSx"; // bootstrap1
const CLIENT_ADDRESS: &str = "tz1gjaF81ZRRvdzjobyfVNsAeSC6PScjfQwN"; // bootstrap2

fn init_node(cfg: &Config) -> Result<()> {
    // 1. Initialize the octez-node configuration
    info!("Initializing octez-node configuration...");
    cfg.octez_node()?.config_init(
        "sandbox",
        &format!("127.0.0.1:{}", SANDBOX_OCTEZ_NODE_PORT),
        &format!("127.0.0.1:{}", SANDBOX_OCTEZ_NODE_RPC_PORT),
        0,
    )?;
    info!(" done");

    // 2. Generate an identity
    info!("Generating identity...");
    cfg.octez_node()?.generate_identity()?;
    info!("done");
    Ok(())
}

fn start_node(cfg: &Config) -> Result<Child> {
    // Run the octez-node in sandbox mode
    let log_file = File::create(node_log_path()?)?;

    cfg.octez_node()?.run(
        &log_file,
        &[
            "--synchronisation-threshold",
            "0",
            "--network",
            "sandbox",
            "--sandbox",
            SANDBOX_PATH,
        ],
    )
}

fn is_node_running(cfg: &Config) -> Result<bool> {
    Ok(cfg
        .octez_client()?
        .rpc(&["get", "/chains/main/blocks/head/hash"])
        .is_ok())
}

fn wait_for_node_to_initialize(cfg: &Config) -> Result<()> {
    if is_node_running(cfg)? {
        return Ok(());
    }

    info!("Waiting for node to initialize...");
    while !is_node_running(cfg)? {
        sleep(Duration::from_secs(1));
        info!(".")
    }

    info!(" done");
    Ok(())
}

fn init_client(cfg: &Config) -> Result<()> {
    // 1. Wait for the node to initialize
    wait_for_node_to_initialize(cfg)?;

    // 2. Wait for the node to be bootstrapped
    info!("Waiting for node to bootstrap...");
    cfg.octez_client()?.wait_for_node_to_bootstrap()?;
    info!(" done");

    // 3. Import activator and bootstrap accounts
    info!("Importing activator account...");
    cfg.octez_client()?
        .import_secret_key("activator", ACTIVATOR_ACCOUNT_SK)?;
    info!("done");

    // 4. Activate alpha
    info!("Activating alpha...");
    cfg.octez_client()?.activate_protocol(
        "ProtoALphaALphaALphaALphaALphaALphaALphaALphaDdp3zK",
        "1",
        "activator",
        SANDBOX_PARAMS_PATH,
    )?;
    info!("done");

    // 5. Import bootstrap accounts
    for (i, sk) in BOOTSTRAP_ACCOUNT_SKS.iter().enumerate() {
        let name = format!("bootstrap{}", i + 1);
        info!("Importing account {}:{}", name, sk);
        cfg.octez_client()?.import_secret_key(&name, sk)?
    }

    Ok(())
}

fn client_bake(cfg: &Config, log_file: &File) -> Result<()> {
    // SAFETY: When a baking fails, then we want to silently ignore the error and
    // try again later since the `client_bake` function is looped in the `OctezThread`.
    let _ = cfg
        .octez_client()?
        .bake(log_file, &["for", "--minimal-timestamp"]);
    Ok(())
}

async fn run_jstz_node() -> Result<()> {
    let local = task::LocalSet::new();

    local
        .run_until(async {
            task::spawn_local(async {
                println!("Jstz node started 🎉");

                jstz_node::run(
                    SANDBOX_JSTZ_NODE_ADDR,
                    SANDBOX_JSTZ_NODE_PORT,
                    &format!(
                        "http://{}:{}",
                        SANDBOX_OCTEZ_SMART_ROLLUP_ADDR, SANDBOX_OCTEZ_SMART_ROLLUP_PORT
                    ),
                    &logs_dir()?.join("kernel.log"),
                )
                .await
            })
            .await
        })
        .await??;

    Ok(())
}

fn start_sandbox(cfg: &Config) -> Result<(OctezThread, OctezThread, OctezThread)> {
    // 1. Init node
    init_node(cfg)?;

    // 2. As a thread, start node
    info!("Starting node...");
    let node = OctezThread::from_child(start_node(cfg)?);
    info!("done");

    // 3. Init client
    init_client(cfg)?;
    info!("Client initialized");

    // 4. As a thread, start baking
    info!("Starting baker...");
    let client_logs = File::create(client_log_path()?)?;
    let baker = OctezThread::new(cfg.clone(), move |cfg| {
        client_bake(cfg, &client_logs)?;
        Ok(())
    });
    info!(" done");

    // 5. Deploy bridge
    info!("Deploying bridge...");

    let ctez_bootstrap_accounts = &[BootstrapAccount {
        address: String::from(CLIENT_ADDRESS),
        amount: 100000000,
    }];

    let ctez_address = deploy_ctez_contract(
        &cfg.octez_client()?,
        OPERATOR_ADDRESS,
        ctez_bootstrap_accounts.iter(),
    )?;

    let bridge =
        BridgeContract::deploy(&cfg.octez_client()?, OPERATOR_ADDRESS, &ctez_address)?;

    info!("done");
    info!("\t`jstz_bridge` deployed at {}", bridge);

    // 6. Create an installer kernel
    info!("Creating installer kernel...");

    let preimages_dir = TempDir::with_prefix("jstz_sandbox_preimages")?.into_path();

    let installer = make_installer(Path::new(JSTZ_KERNEL_PATH), &preimages_dir, &bridge)?;
    info!("done");

    // 7. Originate the rollup
    let rollup = JstzRollup::deploy(&cfg.octez_client()?, OPERATOR_ADDRESS, &installer)?;

    info!("`jstz_rollup` originated at {}", rollup);

    // 8. As a thread, start rollup node
    info!("Starting rollup node...");

    let logs_dir = logs_dir()?;
    let rollup_node = OctezThread::from_child(rollup.run(
        &cfg.octez_rollup_node()?,
        OPERATOR_ADDRESS,
        &preimages_dir,
        &logs_dir,
        "127.0.0.1",
        SANDBOX_OCTEZ_SMART_ROLLUP_PORT,
    )?);
    info!("done");

    // 9. Set the rollup address in the bridge
    bridge.set_rollup(&cfg.octez_client()?, OPERATOR_ADDRESS, &rollup)?;
    info!("\t`jstz_bridge` `rollup` address set to {}", rollup);

    info!("Bridge deployed");

    Ok((node, baker, rollup_node))
}

pub async fn main(cfg: &mut Config) -> Result<()> {
    // 1. Check if sandbox is already running
    if cfg.sandbox.is_some() {
        bail_user_error!("The sandbox is already running!");
    }

    // 1. Configure sandbox
    info!("Configuring sandbox...");
    let sandbox_cfg = SandboxConfig {
        pid: std::process::id(),
        octez_client_dir: TempDir::with_prefix("octez_client")?.into_path(),
        octez_node_dir: TempDir::with_prefix("octez_node")?.into_path(),
        octez_rollup_node_dir: TempDir::with_prefix("octez_rollup_node")?.into_path(),
    };

    // Create logs directory
    fs::create_dir_all(logs_dir()?)?;

    cfg.sandbox = Some(sandbox_cfg);
    info!("done");

    // 2. Start sandbox
    let (node, baker, rollup_node) = start_sandbox(cfg)?;
    info!("Sandbox started 🎉");

    // 3. Save config
    info!("Saving sandbox config");
    cfg.save()?;

    // 4. Wait for the sandbox or jstz-node to shutdown (either by the user or by an error)
    run_jstz_node().await?;
    OctezThread::join(vec![baker, rollup_node, node])?;

    cfg.sandbox = None;
    cfg.save()?;
    Ok(())
}
