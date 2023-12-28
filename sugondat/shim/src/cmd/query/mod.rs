use crate::{
    cli::query::{Commands, Params},
    sugondat_rpc,
};

mod submit;

pub async fn run(params: Params) -> anyhow::Result<()> {
    match params.command {
        Commands::Submit(params) => submit::run(params).await?,
    }
    Ok(())
}

async fn connect_rpc(
    conn_params: crate::cli::SugondatRpcParams,
) -> anyhow::Result<sugondat_rpc::Client> {
    sugondat_rpc::Client::new(conn_params.node_url).await
}
