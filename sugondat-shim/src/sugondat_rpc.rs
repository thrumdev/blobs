use std::sync::Arc;

use crate::key::Keypair;
use anyhow::Context;
use subxt::{backend::rpc::RpcClient, rpc_params, utils::H256, OnlineClient};
use sugondat_nmt::Namespace;
use sugondat_subxt::{
    sugondat::runtime_types::bounded_collections::bounded_vec::BoundedVec, Header,
};
use tokio::sync::watch;

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
    finalized: Arc<FinalizedHeadWatcher>,
}

impl Client {
    /// Creates a new instance of the client.
    pub async fn new(rpc_url: String) -> anyhow::Result<Self> {
        let raw = RpcClient::from_url(&rpc_url).await?;
        let subxt = sugondat_subxt::Client::from_rpc_client(raw.clone()).await?;
        let finalized = Arc::new(FinalizedHeadWatcher::spawn(subxt.clone()).await);
        Ok(Self {
            raw,
            subxt,
            finalized,
        })
    }

    /// Blocks until the sugondat node has finalized a block at the given height. Returns
    /// the block hash of the block at the given height.
    pub async fn wait_finalized_height(&self, height: u64) -> [u8; 32] {
        self.finalized.wait_until_finalized(self, height).await
    }

    /// Returns the block hash of the block at the given height.
    ///
    /// If there is no block at the given height, returns `None`.
    pub async fn block_hash(&self, height: u64) -> Result<Option<H256>, anyhow::Error> {
        let block_hash: H256 = self
            .raw
            .request("chain_getBlockHash", rpc_params![height])
            .await?;
        if block_hash == H256::zero() {
            // Little known fact: the sugondat node returns a zero block hash if there is no block
            // at the given height.
            return Ok(None);
        } else {
            Ok(Some(block_hash))
        }
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
            .with_context(|| format!("failed to validate or sign extrinsic with dev key pair"))?;

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
fn tree_root(header: &Header) -> Option<sugondat_nmt::TreeRoot> {
    let mut nmt_root = None;
    for log in &header.digest.logs {
        match log {
            subxt::config::substrate::DigestItem::Other(ref bytes) => {
                if bytes.starts_with(b"snmt") {
                    nmt_root = Some(sugondat_nmt::TreeRoot::from_raw_bytes(
                        bytes[4..].try_into().unwrap(),
                    ));
                    break;
                }
            }
            _ => {}
        }
    }
    nmt_root
}

/// A small gadget that watches the finalized block headers and remembers the last one.
struct FinalizedHeadWatcher {
    /// The last finalized block header watch value.
    ///
    /// Initialized with 0 as a dummy value.
    rx: watch::Receiver<(u64, [u8; 32])>,
    /// The join handle of the task that watches the finalized block headers.
    handle: tokio::task::JoinHandle<()>,
}

impl FinalizedHeadWatcher {
    /// Spawns the watch task.
    async fn spawn(subxt: sugondat_subxt::Client) -> Self {
        let (tx, rx) = watch::channel((0, [0; 32]));
        let handle = tokio::spawn({
            async move {
                let mut first_attempt = true;
                'resubscribe: loop {
                    if !first_attempt {
                        first_attempt = false;
                        tracing::debug!("resubscribing to finalized block headers in 1 sec");
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                    let Ok(mut stream) = subxt.backend().stream_finalized_block_headers().await
                    else {
                        continue 'resubscribe;
                    };
                    while let Some(header) = stream.next().await {
                        let Ok((header, block_ref)) = header else {
                            continue 'resubscribe;
                        };
                        let _ = tx.send((header.number as u64, block_ref.hash().0));
                    }
                }
            }
        });
        Self { rx, handle }
    }

    /// Wait until the sugondat node has finalized a block at the given height. Returns the block
    /// hash of that finalized block.
    async fn wait_until_finalized(&self, client: &Client, height: u64) -> [u8; 32] {
        let mut rx = self.rx.clone();
        let (finalized_height, block_hash) = loop {
            if let Err(_) = rx.changed().await {
                // The sender half was dropped, meaning something happened to the watch task.
                // It's either shutdown or panicked. Here we hold a reference to `self` meaning
                // that no it ain't shutdown. Therefore, it must have panicked but it's not supposed
                // to, so we cascade the panic.
                panic!("finalized block header watcher task has died")
            }
            let (finalized_height, block_hash) = *rx.borrow();
            if finalized_height < height {
                continue;
            }
            break (finalized_height, block_hash);
        };
        if finalized_height == height {
            // The common case: the finalized block is at the height we're looking for.
            block_hash
        } else {
            // The finalized block is already past the height we're looking for, but we need to
            // return the block hash of the block at the given height. Therefore, we query it.
            loop {
                let Ok(Some(block_hash)) = client.block_hash(height).await else {
                    continue;
                };
                break block_hash.0;
            }
        }
    }
}

impl Drop for FinalizedHeadWatcher {
    fn drop(&mut self) {
        self.handle.abort();
    }
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
