//! Defines the `RelayerBuilder` struct that is used to build the relayer server.

use futures::future;

use crate::cli::config::{ModuleVariant, RelayerConfig};

use super::modules::ModuleServer;
use crate::modules::cosmos_to_eth::CosmosToEthRelayerModule;

// TODO: something fancier than static fields for each supported module?
//  with options for modules from different crates idk
//  this also weirdly ties config and modules together...

/// The `RelayerBuilder` struct is used to build the relayer binary.
#[derive(Default)]
#[allow(clippy::module_name_repetitions)]
pub struct RelayerBuilder;

impl RelayerBuilder {
    /// Start the relayer server.
    #[allow(clippy::pedantic)]
    pub async fn start(self, config: RelayerConfig) -> anyhow::Result<()> {
        // Ensure the starting port and address are set
        let address = config.server.address;

        // Vector to store spawned tasks for each module
        let mut tasks = Vec::new();

        // Iterate through all registered modules
        for module_config in config.modules.into_iter() {
            match module_config {
                ModuleVariant::CosmosToEth(config) => {
                    if !config.enabled {
                        continue;
                    }

                    let port = config.port;
                    let socket_addr = format!("{}:{}", address, port);
                    let socket_addr = socket_addr
                        .parse::<std::net::SocketAddr>()
                        .unwrap_or_else(|err| panic!("Failed to parse socket address: {}", err));

                    tasks.push(tokio::spawn(async move {
                        let module = CosmosToEthRelayerModule;
                        if let Err(err) = module.serve(config, socket_addr).await {
                            tracing::error!(%err, "Failed to start cosmos to eth module");
                        }
                    }));
                }
                ModuleVariant::EthToCosmos(config) => {
                    if !config.enabled {
                        continue;
                    }
                }
            }
        }

        // Wait for all tasks to complete
        future::try_join_all(tasks).await?;

        Ok(())
    }
}
