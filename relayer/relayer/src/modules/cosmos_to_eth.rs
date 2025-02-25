//! Defines Cosmos to Ethereum relayer module.

use std::{net::SocketAddr, str::FromStr};

use alloy::sol_types::SolValue;
use alloy::{
    primitives::Address,
    providers::{ProviderBuilder, RootProvider},
    transports::BoxTransport,
};
use alloy_network::ReceiptResponse;
use ibc_eureka_relayer_lib::listener::{cosmos_sdk, eth_eureka};
use ibc_eureka_solidity_types::sp1_ics07::{
    sp1_ics07_tendermint, ISP1Msgs::SP1Proof, IUpdateClientMsgs::MsgUpdateClient,
};
use prover_client::CelestiaProverClient;
use tendermint_rpc::{HttpClient, Url};
use tonic::{
    transport::{self, Server},
    Request, Response,
};

use crate::cli::config::CosmosToEthConfig;
use crate::{
    api::{
        self,
        relayer_service_server::{RelayerService, RelayerServiceServer},
    },
    core::modules::ModuleServer,
};

/// The `CosmosToEthRelayerModule` struct defines the Cosmos to Ethereum relayer module.
#[derive(Clone, Copy, Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct CosmosToEthRelayerModule;

/// The `CosmosToEthRelayerModuleServer` defines the relayer server from Cosmos to Ethereum.
struct CosmosToEthRelayerModuleServer {
    /// The chain listener for Cosmos SDK.
    pub tm_listener: cosmos_sdk::ChainListener,
    /// The chain listener for `EthEureka`.
    pub eth_listener: eth_eureka::ChainListener<BoxTransport, RootProvider<BoxTransport>>,

    /// Address of the ICS07 Tendermint contract
    pub ics07_address: Address,

    /// prover client connection
    pub prover_client: CelestiaProverClient<transport::Channel>,
    // /// The transaction builder for `EthEureka`.
    //pub tx_builder: TxBuilder<BoxTransport, RootProvider<BoxTransport>>,
    eth_provider: RootProvider<BoxTransport>,
}

/// The configuration for the Cosmos to Ethereum relayer module.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[allow(clippy::module_name_repetitions)]
pub struct CosmosToEthArgs {
    /// The tendermint RPC URL.
    pub tm_rpc_url: String,
    /// The ICS26 address.
    pub ics26_address: Address,
    /// The EVM RPC URL.
    pub eth_rpc_url: String,
    /// The ICS07 address
    pub ics07_address: Address,
    /// The SP1 prover network private key.
    pub prover_url: String,
    //pub wallet: EthereumWallet,
}

impl CosmosToEthRelayerModuleServer {
    async fn new(args: CosmosToEthArgs) -> Self {
        let tm_client = HttpClient::new(
            Url::from_str(&args.tm_rpc_url)
                .unwrap_or_else(|_| panic!("invalid tendermint RPC URL: {}", args.tm_rpc_url)),
        )
        .expect("Failed to create tendermint HTTP client");

        let tm_listener = cosmos_sdk::ChainListener::new(tm_client.clone());

        // TODO: probably needs wallet
        let provider = ProviderBuilder::new()
            .on_builtin(&args.eth_rpc_url)
            .await
            .unwrap_or_else(|e| panic!("failed to create provider: {e}"));

        let eth_listener = eth_eureka::ChainListener::new(args.ics26_address, provider.clone());

        // XXX: align errors
        let prover_client = CelestiaProverClient::with_url(args.prover_url).unwrap();

        Self {
            tm_listener,
            eth_listener,
            ics07_address: args.ics07_address,
            prover_client,
            eth_provider: provider,
        }
    }
}

#[tonic::async_trait]
impl RelayerService for CosmosToEthRelayerModuleServer {
    #[tracing::instrument(skip_all)]
    async fn info(
        &self,
        _request: Request<api::InfoRequest>,
    ) -> Result<Response<api::InfoResponse>, tonic::Status> {
        tracing::info!("Received info request.");
        Ok(Response::new(api::InfoResponse {
            target_chain: Some(api::Chain {
                chain_id: self
                    .eth_listener
                    .chain_id()
                    .await
                    .map_err(|e| tonic::Status::from_error(e.to_string().into()))?,
                ibc_version: "2".to_string(),
                ibc_contract: "TODO".to_string(), //self.tx_builder.ics26_router.address().to_string(),
            }),
            source_chain: Some(api::Chain {
                chain_id: self
                    .tm_listener
                    .chain_id()
                    .await
                    .map_err(|e| tonic::Status::from_error(e.to_string().into()))?,
                ibc_version: "2".to_string(),
                ibc_contract: String::new(),
            }),
        }))
    }

    async fn update_client(
        &self,
        _request: Request<api::UpdateClientRequest>,
    ) -> Result<Response<api::UpdateClientResponse>, tonic::Status> {
        tracing::info!("Handling update client request for cosmos to eth");

        let prover_info = self
            .prover_client
            .info()
            .await
            .map_err(|e| tonic::Status::from_error(e.to_string().into()))?;

        let transition_proof = self
            .prover_client
            .prove_state_transition(self.ics07_address)
            .await
            .map_err(|e| tonic::Status::from_error(e.to_string().into()))?;

        tracing::debug!("communed with prover: {prover_info:?}, {transition_proof:?}");

        let update_msg = MsgUpdateClient {
            sp1Proof: SP1Proof::new(
                &prover_info.state_transition_vkey,
                transition_proof.proof,
                transition_proof.public_values,
            ),
        };

        let contract = sp1_ics07_tendermint::new(self.ics07_address, self.eth_provider.clone());

        let update_receipt = contract
            .updateClient(update_msg.abi_encode().into())
            .send()
            .await
            .map_err(|e| tonic::Status::from_error(e.to_string().into()))?
            .get_receipt()
            .await
            .map_err(|e| tonic::Status::from_error(e.to_string().into()))?;

        // TODO: error on status == 0

        tracing::trace!("Transaction hash: {:?}", update_receipt.block_hash());
        tracing::trace!("Block number: {:?}", update_receipt.block_number());
        tracing::trace!("Gas used: {:?}", update_receipt.gas_used());
        tracing::trace!("Successful: {:?}", update_receipt.status());

        Ok(Response::new(api::UpdateClientResponse {
            tx_hash: update_receipt.transaction_hash().to_string(),
            block_number: update_receipt.block_number(),
            gas_used: update_receipt.gas_used(),
        }))
    }

    #[tracing::instrument(skip_all)]
    async fn relay_by_tx(
        &self,
        _request: Request<api::RelayByTxRequest>,
    ) -> Result<Response<api::RelayByTxResponse>, tonic::Status> {
        tracing::info!("Handling relay by tx request for cosmos to eth...");

        Err(tonic::Status::unimplemented("relaying unimplemented"))
    }
}

#[tonic::async_trait]
impl ModuleServer for CosmosToEthRelayerModule {
    type Config = CosmosToEthConfig;

    #[tracing::instrument(skip_all)]
    async fn serve(
        &self,
        config: Self::Config,
        addr: SocketAddr,
    ) -> Result<(), tonic::transport::Error> {
        tracing::info!("ModuleServer::server");

        let args = CosmosToEthArgs {
            tm_rpc_url: config.tm_rpc_url,
            ics26_address: config.ics26_router,
            ics07_address: config.ics07_tendermint,
            eth_rpc_url: config.eth_rpc_url,
            prover_url: config.prover_url,
        };

        let server = CosmosToEthRelayerModuleServer::new(args).await;

        tracing::info!(%addr, "Started Cosmos to Ethereum relayer server.");

        Server::builder()
            .add_service(RelayerServiceServer::new(server))
            .serve(addr)
            .await
    }
}
