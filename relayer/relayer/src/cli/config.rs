//! Defines the top level configuration for the relayer.

use std::str::FromStr;

use serde_json::Value;
use tracing::Level;

/// The top level configuration for the relayer.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[allow(clippy::module_name_repetitions)]
pub struct RelayerConfig {
    /// The configuration for the relayer modules.
    pub modules: Vec<ModuleVariant>,
    /// The configuration for the relayer server.
    pub server: ServerConfig,
}

/// The configuration for the relayer modules.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[allow(clippy::module_name_repetitions)]
pub struct ModuleConfig {
    /// The name of the module.
    pub name: String,
    /// The port for the RPC server of the module.
    pub port: u16,
    /// The custom configuration for the module.
    pub config: Value,
    /// Whether the module is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "name")]
pub enum ModuleVariant {
    #[serde(alias = "cosmos_to_eth")]
    CosmosToEth(CosmosToEthConfig),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CosmosToEthConfig {
    pub port: u16,
    pub ics26_router: String,
    pub ics02_client: String,
    pub ics07_tendermint: String,
    pub eth_rpc_url: String,
    pub tm_rpc_url: String,
    pub eth_private_key: String,
    //pub sp1_private_key: String,
    pub prover_url: String,
    /// Whether the module is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// The configuration for the relayer server.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[allow(clippy::module_name_repetitions)]
pub struct ServerConfig {
    /// The address to bind the server to.
    pub address: String,
    /// The log level for the server.
    #[serde(default)]
    pub log_level: String,
}

/// Returns true, used as a default value for boolean fields.
const fn default_true() -> bool {
    true
}

impl ServerConfig {
    /// Returns the log level for the server.
    #[must_use]
    pub fn log_level(&self) -> Level {
        Level::from_str(&self.log_level).unwrap_or(Level::INFO)
    }
}
