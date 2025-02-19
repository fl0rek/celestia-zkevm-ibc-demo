use std::path::PathBuf;

use ibc_eureka_relayer_lib::listener::cosmos_sdk::ChainListener;
use ibc_eureka_relayer_lib::listener::ChainListenerService;
use tendermint_rpc::HttpClient;
use alloy::sol_types::SolValue;
use alloy_network::{EthereumWallet, ReceiptResponse};
use alloy_signer_local::PrivateKeySigner;
use alloy::hex;
use alloy::providers::ProviderBuilder;
use celestia_relayer::cli::cmd::{Commands, RelayerCli, ForwardPacketsArgs};
use celestia_relayer::cli::config::{ModuleVariant, RelayerConfig};
use clap::Parser;

use ibc_eureka_solidity_types::sp1_ics07::{
    sp1_ics07_tendermint,
    ISP1Msgs::SP1Proof,
    IUpdateClientMsgs::MsgUpdateClient,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = RelayerCli::parse();

    let config_path = PathBuf::from(cli.config);
    let config_bz = std::fs::read(config_path)?;
    let config: RelayerConfig = serde_json::from_slice(&config_bz)?;
    let ModuleVariant::CosmosToEth(test_config) = &config.modules[0];

    // Initialize the logger with log level.
    tracing_subscriber::fmt::fmt()
        .with_max_level(config.server.log_level())
        .init();

    tracing::info!("cfg: {test_config:#?}");

    match cli.command {
        Commands::UpdateClient => {
            let prover_info = prover_client::info(test_config.prover_url.clone()).await?;
            tracing::info!("prover up: {prover_info:?}");

            //let vkey: [u8; 32] = hex::const_decode_to_array(prover_info.state_transition_vkey.as_bytes())?;

            let transition_proof = prover_client::prove_state_transition(
                test_config.prover_url.clone(),
                test_config.ics07_tendermint.clone(),
            )
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
            let contract = sp1_ics07_tendermint::new(test_config.ics07_tendermint.parse()?, provider);

            let update_receipt = contract.updateClient(update_msg.abi_encode().into())
                .send()
                .await?
                .get_receipt().await?;


            tracing::info!("Transaction hash: {:?}", update_receipt.block_hash());
            tracing::info!("Block number: {:?}", update_receipt.block_number());
            tracing::info!("Gas used: {:?}", update_receipt.gas_used());
            tracing::info!("Successful: {:?}", update_receipt.status());
            /*
            // Build the relayer server.
            let mut relayer_builder = RelayerBuilder::default();
            relayer_builder.add_module(CosmosToEthRelayerModule);
            //relayer_builder.add_module(EthToCosmosRelayerModule);

            // Start the relayer server.
            relayer_builder.start(config).await?;
            */

        }
        Commands::ForwardPackets(ForwardPacketsArgs { from, to }) => {
            let tm_client = HttpClient::new(&*test_config.tm_rpc_url)?;
            let tm_listener = ChainListener::new(tm_client);
            //let ev = tm_listener.fetch_events(from, to).await?;
            let hash = tendermint::hash::Hash::from_hex_upper(
                tendermint::hash::Algorithm::Sha256,
                "77B3C45B0F1201EA2E0A62C50624593CC0C4E9447474BD4E360A08660ACE82F0").unwrap();

            let ev = tm_listener.fetch_tx_events(vec![hash]).await?;

            tracing::info!("evs: {ev:?}");
            

        }
    };
    Ok(())
}

fn wallet_from_key(key: &str) -> anyhow::Result<EthereumWallet> {
    let signer: PrivateKeySigner = key.strip_suffix("0x").unwrap_or(key).parse()?;
    Ok(EthereumWallet::from(signer))
}
