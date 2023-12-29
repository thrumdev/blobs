use super::connect_rpc;
use crate::cli::query::{blob::Params, BlockRef};

use std::io::Write;

pub async fn run(params: Params) -> anyhow::Result<()> {
    let Params {
        rpc,
        block,
        index,
        raw,
    } = params;

    let client = connect_rpc(rpc).await?;

    let maybe_hash = match block {
        BlockRef::Best => None,
        BlockRef::Hash(h) => Some(h),
        BlockRef::Number(n) => Some(
            client
                .block_hash(n)
                .await?
                .ok_or_else(|| anyhow::anyhow!("No block with number {}", n))?
                .0,
        ),
    };

    let block = client.get_block_at(maybe_hash).await?;

    let i = block
        .blobs
        .binary_search_by_key(&index, |b| b.extrinsic_index)
        .map_err(|_| anyhow::anyhow!("No blob with extrinsic index {}", index))?;

    let blob = block.blobs.get(i).expect("verified to exist above; qed");

    if raw {
        std::io::stdout().write_all(&blob.data)?;
    } else {
        println!(
            " Blob #{}, Namespace {}, {} bytes",
            i + 1,
            &blob.namespace,
            blob.data.len()
        );
        println!("{}", hex::encode(&blob.data));
    }

    Ok(())
}
