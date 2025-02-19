mod celestia_tonic {
    tonic::include_proto!("celestia.prover.v1");
}

mod evm_tonic {
    tonic::include_proto!("celestia.ibc.lightclients.groth16.v1");
}

#[derive(Debug)]
pub struct ProverInfo {
    pub state_transition_vkey: String,
    pub state_membership_vkey: String,
}

#[derive(Debug)]
pub struct StateTransitionProof {
    pub proof: Vec<u8>,
    pub public_values: Vec<u8>,
}

pub use celestia_tonic::prover_client::ProverClient;

pub async fn info(url: String) -> anyhow::Result<ProverInfo> {
    let mut client = ProverClient::connect(url).await?;
    let request = tonic::Request::new(celestia_tonic::InfoRequest {});

    let response = client.info(request).await?.into_inner();

    Ok(ProverInfo {
        state_transition_vkey: response.state_transition_verifier_key,
        state_membership_vkey: response.state_membership_verifier_key,
    })
}

pub async fn prove_state_transition(
    url: String,
    client_id: String,
) -> anyhow::Result<StateTransitionProof> {
    let mut client = ProverClient::connect(url).await?;
    let request = tonic::Request::new(celestia_tonic::ProveStateTransitionRequest { client_id });

    let response = client.prove_state_transition(request).await?.into_inner();

    Ok(StateTransitionProof {
        proof: response.proof,
        public_values: response.public_values,
    })
}
