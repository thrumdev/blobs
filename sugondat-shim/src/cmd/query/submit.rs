use anyhow::Context;

use super::connect_rpc;
use crate::cli::query::submit::Params;

pub async fn run(params: Params) -> anyhow::Result<()> {
    let Params {
        blob_path,
        namespace,
        rpc,
    } = params;
    let blob = read_blob(&blob_path)
        .with_context(|| format!("cannot read blob file path '{}'", blob_path))?;
    let namespace = read_namespace(&namespace)?;
    let client = connect_rpc(rpc).await?;
    tracing::info!("submitting blob to namespace {}", namespace);
    let block_hash = client.submit_blob(blob, namespace).await?;
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
/// The original namespace format is a 4-byte vector. so we support both the original format and
/// a more human-readable format, which is an unsigned 32-bit integer. To distinguish between the
/// two, the byte vector must be prefixed with `0x`.
///
/// The integer is interpreted as little-endian.
fn read_namespace(namespace: &str) -> anyhow::Result<sugondat_nmt::Namespace> {
    if namespace.starts_with("0x") {
        let namespace = namespace.trim_start_matches("0x");
        let namespace = hex::decode(namespace)?;
        let namespace: [u8; 4] = namespace.try_into().map_err(|e: Vec<u8>| {
            anyhow::anyhow!("namespace must be 4 bytes long, but was {}", e.len())
        })?;
        Ok(sugondat_nmt::Namespace::from_raw_bytes(namespace))
    } else {
        let namespace_id = namespace
            .parse::<u32>()
            .with_context(|| format!("cannot parse namespace id '{}'", namespace))?;
        Ok(sugondat_nmt::Namespace::with_namespace_id(namespace_id))
    }
}
