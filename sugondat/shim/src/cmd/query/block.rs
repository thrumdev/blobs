use super::connect_rpc;
use crate::cli::query::{block::Params, BlockRef};

pub async fn run(params: Params) -> anyhow::Result<()> {
    let Params { rpc, block } = params;

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

    println!("Block: #{}", block.number);
    println!("  Hash: 0x{}", hex::encode(&block.hash[..]));
    println!("  Parent Hash: 0x{}", hex::encode(&block.parent_hash[..]));
    println!("  Blobs Root: 0x{}", hex::encode(&block.tree_root.root[..]));
    println!("  Min Namespace: {}", block.tree_root.min_ns);
    println!("  Max Namespace: {}", block.tree_root.max_ns);
    println!("  Timestamp: {}", block.timestamp);
    println!(
        "  Blob Count: {} ({} bytes)",
        block.blobs.len(),
        block.blobs.iter().map(|b| b.data.len()).sum::<usize>(),
    );
    for (i, blob) in block.blobs.into_iter().enumerate() {
        println!(" Blob #{}", i + 1);
        println!("    Extrinsic Index: {}", blob.extrinsic_index);
        println!("    Namespace: {}", &blob.namespace);
        println!("    Size: {}", blob.data.len());
    }

    Ok(())
}
