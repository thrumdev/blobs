//! A dock is a component that provides an ad-hoc API consumed by the corresponding adapter in the
//! rollup.

use jsonrpsee::Methods;
use subxt_signer::sr25519::Keypair;

use crate::sugondat_rpc;

mod rollkit;
mod sovereign;

/// A configuration for initializing all docks.
pub struct Config {
    /// Whether or not to enable the sovereign dock.
    pub enable_sovereign: bool,

    /// Whether or not to enable the rollkit dock.
    pub enable_rollkit: bool,

    /// The RPC client handle to the sugondat node.
    pub client: sugondat_rpc::Client,

    /// The optional key used for signing when submitting blobs.
    pub submit_key: Option<Keypair>,
}

/// Initializes all enabled docks and returns the [`Methods`] ready to be registered in the JSON-RPC
/// server.
pub fn init(config: Config) -> Methods {
    let mut methods = Methods::new();
    if config.enable_sovereign {
        sovereign::register(&mut methods, &config);
    }
    if config.enable_rollkit {
        rollkit::register(&mut methods, &config);
    }
    methods
}
