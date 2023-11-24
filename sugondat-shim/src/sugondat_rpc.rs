use crate::key::Keypair;
use anyhow::Context;
use subxt::{backend::rpc::RpcClient, rpc_params, utils::H256, OnlineClient};
use sugondat_nmt::Namespace;
use sugondat_subxt::{
    sugondat::runtime_types::bounded_collections::bounded_vec::BoundedVec, Header,
};

// NOTE: we specifically avoid prolifiration of subxt types around the codebase. To that end, we
//       avoid returning H256 and instead return [u8; 32] directly.

/// A sugondat RPC client.
///
/// # Clone
///
/// This is a thin wrapper that can be cloned cheaply.
#[derive(Clone)]
pub struct Client {
    raw: RpcClient,
    subxt: sugondat_subxt::Client,
}

impl Client {
    /// Creates a new instance of the client.
    pub async fn new(rpc_url: String) -> anyhow::Result<Self> {
        let raw = RpcClient::from_url(&rpc_url).await?;
        let subxt = sugondat_subxt::Client::from_rpc_client(raw.clone()).await?;
        check_if_compatible(&subxt)?;
        Ok(Self { raw, subxt })
    }

    /// Blocks until the sugondat node has finalized a block at the given height.
    pub async fn wait_finalized_height(&self, height: u64) -> anyhow::Result<[u8; 32]> {
        loop {
            let finalized_head = self.subxt.backend().latest_finalized_block_ref().await?;
            let header = self
                .subxt
                .backend()
                .block_header(finalized_head.hash())
                .await?
                .unwrap();
            if header.number as u64 >= height {
                break;
            }
            tracing::info!(
                "waiting for sugondat node to finalize block at height {}",
                height
            );
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        let block_hash: H256 = self
            .raw
            .request("chain_getBlockHash", rpc_params![height])
            .await?;
        Ok(block_hash.0)
    }

    pub async fn get_block_at(&self, block_hash: [u8; 32]) -> anyhow::Result<Block> {
        let block_hash = H256::from(block_hash);
        let block_api = self.subxt.blocks().at(block_hash).await?;
        let timestamp = block_api
            .storage()
            .fetch(&sugondat_subxt::sugondat::storage().timestamp().now())
            .await?
            .ok_or(anyhow::anyhow!("no timestamp found"))?;
        let header = block_api.header();
        let tree_root = tree_root(header).ok_or_else(err::no_tree_root)?;
        let extrinsics = block_api
            .extrinsics()
            .await?
            .iter()
            .collect::<Result<Vec<_>, _>>()?;
        let blobs = extract_blobs(extrinsics);
        tracing::info!(?blobs, "found {} blobs in block", blobs.len());
        Ok(Block {
            number: header.number as u64,
            parent_hash: header.parent_hash.0,
            tree_root,
            timestamp,
            blobs,
        })
    }

    /// Submit a blob with the given namespace and signed with the given key.
    ///
    /// Returns a block hash in which the extrinsic was included.
    pub async fn submit_blob(
        &self,
        blob: Vec<u8>,
        namespace: sugondat_nmt::Namespace,
        key: Keypair,
    ) -> anyhow::Result<[u8; 32]> {
        let namespace_id = namespace.namespace_id();
        let extrinsic = sugondat_subxt::sugondat::tx()
            .blob()
            .submit_blob(namespace_id, BoundedVec(blob));

        let signed = self
            .subxt
            .tx()
            .create_signed(&extrinsic, &key, Default::default())
            .await
            .with_context(|| format!("failed to validate or sign extrinsic"))?;

        let events = signed
            .submit_and_watch()
            .await
            .with_context(|| format!("failed to submit extrinsic"))?
            .wait_for_finalized_success()
            .await?;
        let block_hash = events.block_hash();
        Ok(block_hash.0)
    }
}

/// Tries to find the `Blob` pallet in the runtime metadata. If it's not there, then we are not
/// connected to a Sugondat node.
fn check_if_compatible(client: &sugondat_subxt::Client) -> anyhow::Result<()> {
    assert!(sugondat_subxt::sugondat::PALLETS.contains(&"Blob"));
    if let Some(pallet) = client.metadata().pallet_by_name("Blob") {
        if pallet.call_variant_by_name("submit_blob").is_some() {
            return Ok(());
        }
    }
    Err(anyhow::anyhow!(
        "connected to a Substrate node that is not Sugondat"
    ))
}

/// Iterates over the extrinsics in a block and extracts the submit_blob extrinsics.
fn extract_blobs(
    extrinsics: Vec<
        subxt::blocks::ExtrinsicDetails<
            subxt::SubstrateConfig,
            OnlineClient<subxt::SubstrateConfig>,
        >,
    >,
) -> Vec<Blob> {
    let mut blobs = vec![];
    for (extrinsic_index, e) in extrinsics.iter().enumerate() {
        let Some(sender) = e
            .address_bytes()
            .filter(|a| a.len() == 33)
            .and_then(|a| a[1..].try_into().ok())
        else {
            continue;
        };
        let Ok(Some(submit_blob_extrinsic)) =
            e.as_extrinsic::<sugondat_subxt::sugondat::blob::calls::types::SubmitBlob>()
        else {
            // Not a submit blob extrinsic, skip.
            continue;
        };
        let data = submit_blob_extrinsic.blob.0;
        blobs.push(Blob {
            extrinsic_index: extrinsic_index as u32,
            namespace: sugondat_nmt::Namespace::with_namespace_id(
                submit_blob_extrinsic.namespace_id,
            ),
            sender,
            data,
        })
    }
    blobs
}

/// Examines the header and extracts the tree root committed as one of the logs.
///
/// Returns None if no tree root was found or if the tree root was malformed.
fn tree_root(header: &Header) -> Option<sugondat_nmt::TreeRoot> {
    use subxt::config::substrate::DigestItem;
    let nmt_digest_bytes = header.digest.logs.iter().find_map(|log| match log {
        DigestItem::Other(ref bytes) if bytes.starts_with(b"snmt") => Some(&bytes[4..]),
        _ => None,
    })?;
    let nmt_root: [u8; 68] = nmt_digest_bytes.try_into().ok()?;
    Some(sugondat_nmt::TreeRoot::from_raw_bytes(&nmt_root))
}

mod err {
    pub fn no_tree_root() -> anyhow::Error {
        anyhow::anyhow!("no tree root found in block header. Are you sure this is a sugondat node?")
    }
}

/// Represents a sugondat block.
pub struct Block {
    pub number: u64,
    pub parent_hash: [u8; 32],
    pub tree_root: sugondat_nmt::TreeRoot,
    pub timestamp: u64,
    pub blobs: Vec<Blob>,
}

/// Represents a blob in a sugondat block.
#[derive(Debug)]
pub struct Blob {
    pub extrinsic_index: u32,
    pub namespace: Namespace,
    pub sender: [u8; 32],
    pub data: Vec<u8>,
}

impl Blob {
    pub fn sha2_hash(&self) -> [u8; 32] {
        use sha2::Digest;
        sha2::Sha256::digest(&self.data).into()
    }
}
