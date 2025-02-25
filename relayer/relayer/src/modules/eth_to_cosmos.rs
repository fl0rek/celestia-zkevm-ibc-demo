//! Defines Ethereum to Cosmos relayer module.

use std::{net::SocketAddr, str::FromStr};

use alloy::{
    primitives::Address,
    providers::{ProviderBuilder, RootProvider},
    transports::BoxTransport,
};
use ibc_eureka_relayer_lib::{
    listener::{cosmos_sdk, eth_eureka},
    tx_builder::eth_to_cosmos,
};
use tendermint_rpc::{HttpClient, Url};
use tonic::{transport::Server, Request, Response};

use crate::{
    api::{
        self,
        relayer_service_server::{RelayerService, RelayerServiceServer},
    },
    core::modules::ModuleServer,
};

/// The `CosmosToCosmosRelayerModule` struct defines the Cosmos to Cosmos relayer module.
#[derive(Clone, Copy, Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct EthToCosmosRelayerModule;

/// The `CosmosToCosmosRelayerModuleServer` defines the relayer server from Cosmos to Cosmos.
struct EthToCosmosRelayerModuleServer {
    /// The chain listener for `EthEureka`.
    pub eth_listener: eth_eureka::ChainListener<BoxTransport, RootProvider<BoxTransport>>,
    /// The chain listener for Cosmos SDK.
    pub tm_listener: cosmos_sdk::ChainListener,
    /// The transaction builder for Ethereum to Cosmos.
    pub tx_builder: EthToCosmosTxBuilder,
}

enum EthToCosmosTxBuilder {
    Real(eth_to_cosmos::TxBuilder<BoxTransport, RootProvider<BoxTransport>>),
    Mock(eth_to_cosmos::MockTxBuilder<BoxTransport, RootProvider<BoxTransport>>),
}

/// The configuration for the Cosmos to Cosmos relayer module.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[allow(clippy::module_name_repetitions)]
pub struct EthToCosmosConfig {
    /// The ICS26 address.
    pub ics26_address: Address,
    /// The tendermint RPC URL.
    pub tm_rpc_url: String,
    /// The EVM RPC URL.
    pub eth_rpc_url: String,
    /// The Ethereum Beacon API URL
    pub eth_beacon_api_url: String,
    /// The address of the submitter.
    /// Required since cosmos messages require a signer address.
    pub signer_address: String,
    /// Whether to run in mock mode.
    #[serde(default)]
    pub mock: bool,
}

impl EthToCosmosRelayerModuleServer {
    async fn new(config: EthToCosmosConfig) -> Self {
        let provider = ProviderBuilder::new()
            .on_builtin(&config.eth_rpc_url)
            .await
            .unwrap_or_else(|e| panic!("failed to create provider: {e}"));
        let eth_listener = eth_eureka::ChainListener::new(config.ics26_address, provider.clone());

        let tm_client = HttpClient::new(
            Url::from_str(&config.tm_rpc_url)
                .unwrap_or_else(|_| panic!("invalid tendermint RPC URL: {}", config.tm_rpc_url)),
        )
        .expect("Failed to create tendermint HTTP client");
        let tm_listener = cosmos_sdk::ChainListener::new(tm_client.clone());

        let tx_builder = if config.mock {
            EthToCosmosTxBuilder::Mock(eth_to_cosmos::MockTxBuilder::new(
                config.ics26_address,
                provider,
                config.signer_address,
            ))
        } else {
            EthToCosmosTxBuilder::Real(eth_to_cosmos::TxBuilder::new(
                config.ics26_address,
                provider,
                config.eth_beacon_api_url,
                tm_client,
                config.signer_address,
            ))
        };

        Self {
            eth_listener,
            tm_listener,
            tx_builder,
        }
    }
}

#[tonic::async_trait]
impl RelayerService for EthToCosmosRelayerModuleServer {
    #[tracing::instrument(skip_all)]
    async fn info(
        &self,
        _request: Request<api::InfoRequest>,
    ) -> Result<Response<api::InfoResponse>, tonic::Status> {
        tracing::info!("Received info request.");
        Ok(Response::new(api::InfoResponse {
            target_chain: Some(api::Chain {
                chain_id: self
                    .tm_listener
                    .chain_id()
                    .await
                    .map_err(|e| tonic::Status::from_error(e.to_string().into()))?,
                ibc_version: "2".to_string(),
                ibc_contract: String::new(),
            }),
            source_chain: Some(api::Chain {
                chain_id: self
                    .eth_listener
                    .chain_id()
                    .await
                    .map_err(|e| tonic::Status::from_error(e.to_string().into()))?,
                ibc_version: "2".to_string(),
                ibc_contract: self.tx_builder.ics26_router_address().to_string(),
            }),
        }))
    }

    async fn update_client(
        &self,
        _request: Request<api::UpdateClientRequest>,
    ) -> Result<Response<api::UpdateClientResponse>, tonic::Status> {
        tracing::info!("Handling update client request for eth to cosmos");

        Err(tonic::Status::unimplemented("update client unimplemented"))
    }

    #[tracing::instrument(skip_all)]
    async fn relay_by_tx(
        &self,
        _request: Request<api::RelayByTxRequest>,
    ) -> Result<Response<api::RelayByTxResponse>, tonic::Status> {
        tracing::info!("Handling relay by tx request for eth to cosmos...");

        Err(tonic::Status::unimplemented("relaying unimplemented"))
    }
}

#[tonic::async_trait]
impl ModuleServer for EthToCosmosRelayerModule {
    type Config = EthToCosmosConfig;

    #[tracing::instrument(skip_all)]
    async fn serve(
        &self,
        config: EthToCosmosConfig,
        addr: SocketAddr,
    ) -> Result<(), tonic::transport::Error> {
        let server = EthToCosmosRelayerModuleServer::new(config).await;

        tracing::info!(%addr, "Started Cosmos to Ethereum relayer server.");

        Server::builder()
            .add_service(RelayerServiceServer::new(server))
            .serve(addr)
            .await
    }
}

impl EthToCosmosTxBuilder {
    const fn ics26_router_address(&self) -> &Address {
        match self {
            Self::Real(tb) => tb.ics26_router.address(),
            Self::Mock(tb) => tb.ics26_router.address(),
        }
    }
}
