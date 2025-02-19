//! Defines the client interface for the relayer server.

use clap::{command, Parser};

/// The command line interface for the relayer.
#[derive(Clone, Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct RelayerCli {
    pub config: String,
    /// The subcommand to run.
    #[command(subcommand)]
    pub command: Commands,
}

/// The subcommands for the relayer.
#[derive(Clone, Debug, Parser)]
pub enum Commands {
    /// The subcommand to run the relayer.
    UpdateClient,
    ForwardPackets(ForwardPacketsArgs),
}

#[derive(Clone, Debug, Parser)]
pub struct ForwardPacketsArgs {
    pub from: u32,
    pub to: u32,
}
