use super::{connect_rpc, get_block_at};
use crate::cli::query::blob::Params;

use std::io::Write;

pub async fn run(params: Params) -> anyhow::Result<()> {
    let Params {
        rpc,
        block,
        index,
        raw,
    } = params;

    let client = connect_rpc(rpc).await?;
    let block = get_block_at(&client, block).await?;

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
            i,
            &blob.namespace,
            blob.data.len()
        );
        println!("{}", hex::encode(&blob.data));
    }

    Ok(())
}
