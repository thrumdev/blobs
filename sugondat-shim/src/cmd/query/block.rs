use anyhow::Context;

use super::connect_rpc;
use crate::cli::query::block::Params;

pub async fn run(params: Params) -> anyhow::Result<()> {
    let Params {
        rpc,
        block_spec,
    } = params;
    drop((rpc, block_spec));
    Ok(())
}
