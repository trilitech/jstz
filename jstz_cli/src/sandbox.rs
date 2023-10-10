use crate::config::Config;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::sync::mpsc;
use std::thread;

use crate::sandbox_initializer::init_sandboxed_client;
use crate::sandbox_initializer::start_rollup_node;
use crate::sandbox_initializer::start_sandboxed_node;

fn ensure_empty_directory(dir_path: &str) -> std::io::Result<()> {
    if Path::new(dir_path).exists() {
        fs::remove_dir_all(dir_path)?; // Remove the directory if it exists
    }
    fs::create_dir_all(dir_path) // Create the directory
}

pub fn sandbox_start(cfg: &mut Config) {
    match env::current_dir() {
        Ok(path) => {
            println!("The current directory is: {}", path.display());
        }
        Err(e) => {
            println!("Error getting current directory: {}", e);
        }
    }

    // Check if sandbox is already running
    if cfg.get_is_sandbox_running() {
        println!("Error: Sandbox is already running!");
        return;
    }

    let root_dir = env::current_dir().expect("Failed to get root directory");
    let log_dir = root_dir.join("logs");
    let script_dir = root_dir.clone();
    let target_dir = root_dir.parent().unwrap().join("target");

    let port = 19730;
    let rpc = 18730;

    // Create temporary directories
    let node_dir = "../octez_node";
    let rollup_node_dir = "../octez_smart_rollup_node";
    let client_dir = "../octez_client";

    ensure_empty_directory(node_dir).expect("Could not ensure empty directory.");
    ensure_empty_directory(rollup_node_dir).expect("Could not ensure empty directory.");
    ensure_empty_directory(client_dir).expect("Could not ensure empty directory.");

    cfg.set_octez_client_dir(client_dir.to_string());

    if !Path::new(cfg.get_octez_client_dir()).exists() {
        println!("The octez client file does not exists.");
    }

    let client = format!(
        "{}/octez-client -base-dir {} -endpoint http://127.0.0.1:{}",
        root_dir.to_str().unwrap(),
        client_dir,
        rpc
    );

    let kernel = format!(
        "{}/kernel/jstz_kernel_installer.hex",
        target_dir.to_str().unwrap()
    );
    let preimages = format!("{}/kernel/preimages", target_dir.to_str().unwrap());

    // Check if the file exists before trying to remove it
    if PathBuf::from(&kernel).exists() {
        fs::remove_file(&kernel)
            .expect("Failed to remove jstz_kernel_installer.hex file.");
    }

    // Check if the directory exists before trying to remove it
    if PathBuf::from(&preimages).exists() {
        fs::remove_dir_all(&preimages).expect("Failed to remove preimages subdirectory.");
    }

    fs::create_dir_all(&log_dir).expect("Failed to create log directory");

    cfg.add_pid(process::id());
    cfg.set_is_sandbox_running(true);
    cfg.save_to_file().expect("Failed to save the config");

    // Start the sandboxed node using the CLI

    let (tx, rx): (mpsc::Sender<&str>, mpsc::Receiver<&str>) = mpsc::channel();
    let (tx_node_pid, rx_node_pid): (mpsc::Sender<u32>, mpsc::Receiver<u32>) =
        mpsc::channel();

    let handle1 = thread::spawn({
        let script_dir_clone = script_dir.clone();
        move || {
            let child = start_sandboxed_node(
                &PathBuf::from(&node_dir),
                port,
                rpc,
                &PathBuf::from(&script_dir_clone.to_str().unwrap()),
            );
            tx_node_pid.send(child.unwrap().id()).unwrap();
        }
    });

    // Initialize the sandboxed client using the CLI
    let handle2 = thread::spawn(move || {
        init_sandboxed_client(&client, &PathBuf::from(&script_dir.to_str().unwrap()), tx)
    });

    // Start the rollup node using the CLI
    let handle3 = thread::spawn(move || {
        start_rollup_node(
            &kernel,
            &preimages,
            &PathBuf::from(&rollup_node_dir),
            &PathBuf::from(&log_dir.to_str().unwrap()),
            rx,
        )
    });

    //Save sandboxed node pid
    cfg.add_pid(rx_node_pid.recv().unwrap());
    cfg.save_to_file().expect("Failed to save the config.");

    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();
}

pub fn sandbox_stop(cfg: &mut Config) {
    // Check if sandbox is not running
    if !cfg.get_is_sandbox_running() {
        println!("Error: Sandbox is not running!");
        return;
    }

    // Kill the processes using their PIDs
    let pids = cfg.get_active_pids();
    for pid in pids {
        let pid = Pid::from_raw(pid as i32);
        // Send a termination signal to the process
        let _ = kill(pid, Signal::SIGTERM);
        cfg.remove_pid(pid.as_raw() as u32);
    }

    // Update the is_sandbox_running property
    cfg.set_is_sandbox_running(false);
}
