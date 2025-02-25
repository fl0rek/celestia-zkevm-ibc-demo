use tonic::Status;

pub type Result<T, E = Error> = std::result::Result<T, E>;
/// Representation of all the errors that can occur when interacting with [`prover_client`].
///
/// [`prover_client`]: crate
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Tonic error
    #[error(transparent)]
    TonicError(#[from] Status),

    /// Transport error
    #[error("Transport: {0}")]
    TransportError(String),
}
