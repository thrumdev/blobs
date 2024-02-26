use crate::{
    cli::serve::{self, Dock, Params},
    cmd::read_namespace,
    dock,
    ikura_rpc::Client,
};
use tracing::info;

pub async fn run(Params { dock }: Params) -> anyhow::Result<()> {
    match dock {
        Dock::Sov(params) => run_sov(params).await,
        Dock::Rollkit(params) => run_rollkit(params).await,
    }
}

async fn connect_client(url: &str, no_retry: bool) -> anyhow::Result<Client> {
    let client = Client::new(url.to_string(), no_retry).await?;
    Ok(client)
}

fn load_submit_key(
    params: crate::cli::KeyManagementParams,
) -> Result<Option<subxt_signer::sr25519::Keypair>, anyhow::Error> {
    let submit_key = crate::cmd::load_key(params)?;
    if submit_key.is_none() {
        tracing::info!(
            "no submit key provided, will not be able to submit blobs. \
Pass --submit-dev-alice or --submit-private-key=<..> to fix."
        );
    }
    Ok(submit_key)
}

async fn run_sov(params: serve::sov::Params) -> anyhow::Result<()> {
    info!(
        "starting Sovereign SDK JSON-RPC ikura-shim server on {}:{}",
        params.dock.address, params.dock.port
    );
    let submit_key = load_submit_key(params.key_management)?;
    let client = connect_client(&params.rpc.node_url, params.rpc.no_retry).await?;
    let config = dock::sovereign::Config {
        client,
        submit_key,
        address: params.dock.address,
        port: params.dock.port,
    };
    dock::sovereign::run(config).await?;
    Ok(())
}

async fn run_rollkit(params: serve::rollkit::Params) -> anyhow::Result<()> {
    info!(
        "starting Rollkit SDK gRPC ikura-shim server on {}:{}",
        params.dock.address, params.dock.port
    );
    let submit_key = load_submit_key(params.key_management)?;
    let namespace = params.namespace.map(|ns| read_namespace(&ns)).transpose()?;
    if namespace.is_none() {
        tracing::info!("no namespace provided, will not be able to submit blobs");
    }
    let client = connect_client(&params.rpc.node_url, params.rpc.no_retry).await?;
    let config = dock::rollkit::Config {
        client,
        submit_key,
        address: params.dock.address,
        port: params.dock.port,
        namespace,
    };
    dock::rollkit::run(config).await?;
    Ok(())
}
