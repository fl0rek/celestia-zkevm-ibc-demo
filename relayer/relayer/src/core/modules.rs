//! Defines the [`RelayerModule`] trait that must be implemented by all relayer modules.

use std::{
    marker::{Send, Sync},
    net::SocketAddr,
};

/// The `RelayerModuleServer` trait defines the interface for launching a relayer module server.
#[tonic::async_trait]
pub trait ModuleServer: Send + Sync + 'static {
    /// Configuration for the module server
    type Config;

    /// Serve the relayer module RPC on the given address.
    async fn serve(
        &self,
        config: Self::Config,
        _addr: SocketAddr,
    ) -> Result<(), tonic::transport::Error>;
}
