use crate::{
    dock::sovereign::SovereignDock,
    cli::{serve::Params, DockParams},
    key::Keypair,
    sugondat_rpc::Client,
};

use jsonrpsee::{server::Server, Methods};
use sugondat_shim_common_sovereign::SovereignRPCServer;
use tracing::{debug, info};

pub async fn run(params: Params) -> anyhow::Result<()> {
    info!(
        "starting sugondat-shim server on {}:{}",
        params.dock.address, params.dock.port
    );
    let listen_on = (params.dock.address.as_str(), params.dock.port);
    let maybe_key = crate::cmd::load_key(params.key_management)?;
    let server = Server::builder().build(listen_on).await?;
    let client = connect_client(&params.rpc.node_url).await?;
    let handle = server.start(init_docks(client, &params.dock, maybe_key));
    handle.stopped().await;
    Ok(())
}

async fn connect_client(url: &str) -> anyhow::Result<Client> {
    let client = Client::new(url.to_string()).await?;
    Ok(client)
}

fn init_docks(
    client: Client,
    dock_params: &DockParams,
    maybe_key: Option<Keypair>,
) -> Methods {
    let mut methods = Methods::new();
    if dock_params.enable_sovereign() {
        debug!("enabling sovereign adapter dock");
        let dock = SovereignDock::new(client.clone(), maybe_key);
        methods.merge(dock.into_rpc()).unwrap();
    }
    methods
}
