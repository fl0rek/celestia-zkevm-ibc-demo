use alloy::primitives::Address;
use bytes::Bytes;
use celestia_grpc_macros::grpc_method;
use http_body::Body;
use tonic::body::BoxBody;
use tonic::client::GrpcService;
use tonic::transport::Channel;

/*
mod evm_tonic {
    tonic::include_proto!("celestia.ibc.lightclients.groth16.v1");
}
*/

mod error;
mod grpc;

pub use error::{Error, Result};
use grpc::prover_client::ProverClient;
use grpc::InfoRequest;
use grpc::ProveStateTransitionRequest;

/// Error convertible to std, used by grpc transports
pub type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;

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

pub struct CelestiaProverClient<T> {
    transport: T,
}

impl<T> CelestiaProverClient<T>
where
    T: GrpcService<BoxBody> + Clone,
    T::Error: Into<StdError>,
    T::ResponseBody: Body<Data = Bytes> + Send + 'static,
    <T::ResponseBody as Body>::Error: Into<StdError> + Send,
{
    /// Get prover info
    #[grpc_method(ProverClient::info)]
    async fn info(&self) -> Result<ProverInfo>;

    /// Get client update proof
    #[grpc_method(ProverClient::prove_state_transition)]
    async fn prove_state_transition(&self, client_id: Address) -> Result<StateTransitionProof>;

    // TODO: membership
}

impl CelestiaProverClient<Channel> {
    /// Create a new client connected to the given `url` with default
    /// settings of [`tonic::transport::Channel`].
    pub fn with_url(url: impl Into<String>) -> Result<Self, tonic::transport::Error> {
        let channel = tonic::transport::Endpoint::from_shared(url.into())?.connect_lazy();
        Ok(Self { transport: channel })
    }
}

impl grpc::FromGrpcResponse<ProverInfo> for grpc::InfoResponse {
    fn try_from_response(self) -> Result<ProverInfo> {
        Ok(ProverInfo {
            state_transition_vkey: self.state_transition_verifier_key,
            state_membership_vkey: self.state_membership_verifier_key,
        })
    }
}

impl grpc::FromGrpcResponse<StateTransitionProof> for grpc::ProveStateTransitionResponse {
    fn try_from_response(self) -> Result<StateTransitionProof> {
        let grpc::ProveStateTransitionResponse {
            proof,
            public_values,
        } = self;
        Ok(StateTransitionProof {
            proof,
            public_values,
        })
    }
}

grpc::make_empty_params!(InfoRequest);

impl grpc::IntoGrpcParam<ProveStateTransitionRequest> for Address {
    fn into_parameter(self) -> ProveStateTransitionRequest {
        ProveStateTransitionRequest {
            client_id: self.to_string(),
        }
    }
}

/*
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
*/
