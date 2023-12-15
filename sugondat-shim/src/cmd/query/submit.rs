use anyhow::Context;

use super::connect_rpc;
use crate::cli::query::submit::Params;

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
        .ok_or_else(|| anyhow::anyhow!("submission signing key required"))?;

    let namespace = read_namespace(&namespace)?;
    let client = connect_rpc(rpc).await?;
    tracing::info!("submitting blob to namespace {}", namespace);
    let block_hash = client.submit_blob(blob, namespace, key).await?;
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

/// Reads the namespace from a given namespace specifier.
///
/// The original namespace format is a 16-byte vector. so we support both the original format and
/// a more human-readable format, which is an unsigned 128-bit integer. To distinguish between the
/// two, the byte vector must be prefixed with `0x`.
///
/// The integer is interpreted as big-endian.
fn read_namespace(namespace: &str) -> anyhow::Result<sugondat_nmt::Namespace> {
    if let Some(hex) = namespace.strip_prefix("0x") {
        let namespace = hex::decode(hex)?;
        let namespace: [u8; 16] = namespace.try_into().map_err(|e: Vec<u8>| {
            anyhow::anyhow!("namespace must be 16 bytes long, but was {}", e.len())
        })?;
        return Ok(sugondat_nmt::Namespace::from_raw_bytes(namespace));
    }

    let namespace_id = namespace
        .parse::<u128>()
        .with_context(|| format!("cannot parse namespace id '{}'", namespace))?;
    Ok(sugondat_nmt::Namespace::from_u128_be(namespace_id))
}
