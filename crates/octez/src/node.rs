use std::{
    fs::File,
    path::PathBuf,
    process::{Child, Command, Stdio},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{run_command, OctezSetup};

#[derive(Debug, Serialize, Deserialize)]
pub struct OctezNode {
    /// Path to the octez-node binary
    /// If None, the binary will inside PATH will be used
    pub octez_setup: Option<OctezSetup>,
    /// Path to the octez-node directory
    pub octez_node_dir: PathBuf,
}

impl OctezNode {
    /// Create a command based on the octez setup configuration
    fn command(&self) -> Command {
        match &self.octez_setup {
            Some(OctezSetup::Process(path)) => {
                let bin_path = path.join("octez-node");
                Command::new(bin_path)
            }
            Some(OctezSetup::Docker(container_name)) => {
                let mut cmd = Command::new("docker");
                cmd.args(["exec", container_name, "octez-node"]);
                cmd
            }
            None => Command::new("octez-node"), // Default to using the system's octez-node
        }
    }

    pub fn config_init(
        &self,
        network: &str,
        http_endpoint: &str,
        rpc_endpoint: &str,
        allow_all_rpc: &str,
        num_connections: u32,
    ) -> Result<()> {
        run_command(self.command().args([
            "config",
            "init",
            "--network",
            network,
            "--data-dir",
            self.octez_node_dir.to_str().expect("Invalid path"),
            "--net-addr",
            http_endpoint,
            "--rpc-addr",
            rpc_endpoint,
            "--allow-all-rpc",
            allow_all_rpc,
            "--connections",
            num_connections.to_string().as_str(),
        ]))
    }

    pub fn generate_identity(&self) -> Result<()> {
        run_command(self.command().args([
            "identity",
            "generate",
            "--data-dir",
            self.octez_node_dir.to_str().expect("Invalid path"),
        ]))
    }

    pub fn run(&self, log_file: &File, options: &[&str]) -> Result<Child> {
        let mut command = self.command();

        command
            .args([
                "run",
                "--data-dir",
                self.octez_node_dir.to_str().expect("Invalid path"),
                "--singleprocess",
            ])
            .args(options)
            .stdout(Stdio::from(log_file.try_clone()?))
            .stderr(Stdio::from(log_file.try_clone()?));

        Ok(command.spawn()?)
    }
}
