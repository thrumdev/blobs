use anyhow::Context;

use super::connect_rpc;
use crate::{cli::query::submit::Params, cmd::read_namespace};

pub async fn run(params: Params) -> anyhow::Result<()> {
    let Params {
        blob_path,
        namespace,
        rpc,
        key_management,
    } = params;
    let blob = read_blob(&blob_path)
        .with_context(|| format!("cannot read blob file path '{}'", blob_path))?;

    let key = crate::cmd::load_key(key_management)
        .with_context(|| format!("cannot load submission signing key"))?
        .ok_or_else(|| anyhow::anyhow!("submission signing key required. Specify the key with --submit-private-key or use the dev key with --submit-dev-alice"))?;

    let namespace = read_namespace(&namespace)?;
    let client = connect_rpc(rpc).await?;
    tracing::info!("submitting blob to namespace {}", namespace);
    let (block_hash, _) = client.submit_blob(blob, namespace, key).await?;
    tracing::info!("submitted blob to block hash 0x{}", hex::encode(block_hash));
    Ok(())
}

/// Reads a blob from either a file or stdin.
fn read_blob(path: &str) -> anyhow::Result<Vec<u8>> {
    use std::io::Read;
    let mut blob = Vec::new();
    if path == "-" {
        tracing::debug!("reading blob contents from stdin");
        std::io::stdin().read_to_end(&mut blob)?;
    } else {
        std::fs::File::open(path)?.read_to_end(&mut blob)?;
    }
    Ok(blob)
}
