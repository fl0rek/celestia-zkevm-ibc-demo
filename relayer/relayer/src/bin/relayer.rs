use std::path::PathBuf;

use alloy::providers::ProviderBuilder;
use alloy::sol_types::SolValue;
use alloy_network::ReceiptResponse;
use celestia_relayer::cli::cmd::{Commands, ForwardPacketsArgs, RelayerCli};
use celestia_relayer::cli::config::{ModuleVariant, RelayerConfig};
use celestia_relayer::core::builder::RelayerBuilder;
use celestia_relayer::modules::cosmos_to_eth::wallet_from_key;
use clap::Parser;
use ibc_eureka_relayer_lib::listener::cosmos_sdk::ChainListener;
use ibc_eureka_relayer_lib::listener::ChainListenerService;
use tendermint_rpc::HttpClient;

use ibc_eureka_solidity_types::sp1_ics07::{
    sp1_ics07_tendermint, ISP1Msgs::SP1Proof, IUpdateClientMsgs::MsgUpdateClient,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = RelayerCli::parse();

    let config_path = PathBuf::from(cli.config);
    let config_bz = std::fs::read(config_path)?;
    let config: RelayerConfig = serde_json::from_slice(&config_bz)?;

    // Initialize the logger with log level.
    tracing_subscriber::fmt::fmt()
        .with_max_level(config.server.log_level())
        .init();

    match cli.command {
        Commands::Start => {
            let relayer = RelayerBuilder;
            relayer.start(config).await?;
        }
        Commands::UpdateClient => {
            // TODO: this assumes config has cosmos_to_eth config in the first slot
            // remote this entirely or update to work in general
            let ModuleVariant::CosmosToEth(test_config) = &config.modules[0] else {
                panic!("I'm sorry Maciek");
            };
            tracing::info!("cfg: {test_config:#?}");

            let prover_client =
                prover_client::CelestiaProverClient::with_url(test_config.prover_url.clone())?;
            let prover_info = prover_client.info().await?;
            tracing::info!("prover up: {prover_info:?}");

            let transition_proof = prover_client
                .prove_state_transition(test_config.ics07_tendermint)
                .await?;

            tracing::info!("proof ok");

            let update_msg = MsgUpdateClient {
                sp1Proof: SP1Proof::new(
                    &prover_info.state_transition_vkey,
                    transition_proof.proof,
                    transition_proof.public_values,
                ),
            };

            let wallet = wallet_from_key(&test_config.eth_private_key)?;
            let provider = ProviderBuilder::new()
                .with_recommended_fillers()
                .wallet(wallet)
                .on_http(test_config.eth_rpc_url.parse()?);
            let contract = sp1_ics07_tendermint::new(test_config.ics07_tendermint, provider);

            let update_receipt = contract
                .updateClient(update_msg.abi_encode().into())
                .send()
                .await?
                .get_receipt()
                .await?;

            tracing::info!("Transaction hash: {:?}", update_receipt.block_hash());
            tracing::info!("Block number: {:?}", update_receipt.block_number());
            tracing::info!("Gas used: {:?}", update_receipt.gas_used());
            tracing::info!("Successful: {:?}", update_receipt.status());
        }
        Commands::ForwardPackets(ForwardPacketsArgs { from, to }) => {
            // TODO: ditto here, oneshot test code
            let ModuleVariant::CosmosToEth(test_config) = &config.modules[0] else {
                panic!("I'm sorry Maciek");
            };
            tracing::info!("cfg: {test_config:#?}");
            let tm_client = HttpClient::new(&*test_config.tm_rpc_url)?;
            let tm_listener = ChainListener::new(tm_client);
            let ev = tm_listener.fetch_events(from, to).await?;
            tracing::info!("evs: {ev:?}");

            let hash = tendermint::hash::Hash::from_hex_upper(
                tendermint::hash::Algorithm::Sha256,
                "CC1CB1C41FC0217DCC500D8285DEEDBD9587CE5671B41E2A340B5FFFD5C93C4F", //"E86658A0640DA0FF62AEF1BC7B8F8C396A5C4D6E41F41E145022D833CAA079A8"
                                                                                    //"77B3C45B0F1201EA2E0A62C50624593CC0C4E9447474BD4E360A08660ACE82F0"
            )
            .unwrap();

            let ev = tm_listener.fetch_tx_events(vec![hash]).await?;

            tracing::info!("evs: {ev:?}");
        }
    };
    Ok(())
}
