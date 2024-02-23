use crate::{
    cli::query::{BlockParams, BlockRef, Commands, Params},
    ikura_rpc,
};

mod blob;
mod block;
mod submit;

pub async fn run(params: Params) -> anyhow::Result<()> {
    match params.command {
        Commands::Submit(params) => submit::run(params).await?,
        Commands::Block(params) => block::run(params).await?,
        Commands::Blob(params) => blob::run(params).await?,
    }
    Ok(())
}

/// Given the BlockParams and the client to be used, try to fetch
/// the corresponding block. It will wait until the block is avaiable if specified.
pub async fn get_block_at(
    client: &ikura_rpc::Client,
    block: BlockParams,
) -> anyhow::Result<ikura_rpc::Block> {
    let maybe_hash = match block.block_ref {
        None | Some(BlockRef::Best) => None,
        Some(BlockRef::Hash(h)) => Some(h),
        Some(BlockRef::Number(n)) => Some(match block.wait {
            true => client.await_block_hash(n).await,
            false => client
                .block_hash(n)
                .await?
                .ok_or_else(|| anyhow::anyhow!("No block with number {}", n))?,
        }),
    };

    match block.wait {
        true => client.await_block_at(maybe_hash).await,
        false => client.get_block_at(maybe_hash).await,
    }
}

async fn connect_rpc(conn_params: crate::cli::IkuraRpcParams) -> anyhow::Result<ikura_rpc::Client> {
    ikura_rpc::Client::new(conn_params.node_url, conn_params.no_retry).await
}
