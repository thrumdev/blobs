use jsonrpsee::{types::ErrorObjectOwned, Methods};
use sugondat_shim_common_sovereign::{Block, SovereignRPCServer};
use tracing::{debug, info};

use super::rpc_error as err;
use crate::{key::Keypair, sugondat_rpc};

/// Registers the sovereign dock in the given methods.
pub fn register(methods: &mut Methods, config: &super::Config) {
    debug!("enabling sovereign adapter dock");
    let dock = SovereignDock::new(config.client.clone(), config.submit_key.clone());
    methods
        .merge(dock.into_rpc())
        .expect("adapter namespace must be unique");
}

struct SovereignDock {
    client: sugondat_rpc::Client,
    submit_key: Option<Keypair>,
}

impl SovereignDock {
    fn new(client: sugondat_rpc::Client, submit_key: Option<Keypair>) -> Self {
        Self { client, submit_key }
    }
}

#[async_trait::async_trait]
impl SovereignRPCServer for SovereignDock {
    async fn get_block(
        &self,
        height: u64,
        namespace: sugondat_nmt::Namespace,
    ) -> Result<Block, ErrorObjectOwned> {
        info!("get_block({})", height);
        let block_hash = self.client.await_finalized_height(height).await;
        let block = self.client.await_block_at(Some(block_hash)).await.unwrap();
        let proof = make_namespace_proof(&block, namespace);
        let blobs = block
            .blobs
            .into_iter()
            .filter(|blob| blob.namespace == namespace)
            .map(|blob| sugondat_shim_common_sovereign::Blob {
                sender: blob.sender,
                data: blob.data,
            })
            .collect::<Vec<_>>();
        Ok(Block {
            block_hash,
            prev_hash: block.parent_hash,
            timestamp: block.timestamp,
            nmt_root: block.tree_root,
            proof,
            blobs,
        })
    }

    async fn submit_blob(
        &self,
        blob: Vec<u8>,
        namespace: sugondat_nmt::Namespace,
    ) -> Result<(), ErrorObjectOwned> {
        info!("submit_blob({}, {:?})", blob.len(), namespace);
        let submit_key = self
            .submit_key
            .as_ref()
            .cloned()
            .ok_or_else(err::no_signing_key)?;
        self.client
            .submit_blob(blob, namespace, submit_key)
            .await
            .map_err(err::submission_error)?;
        Ok(())
    }
}

/// Creates a namespace proof for the given namespace in the given block.
fn make_namespace_proof(
    block: &sugondat_rpc::Block,
    namespace: sugondat_nmt::Namespace,
) -> sugondat_nmt::NamespaceProof {
    let mut blob_metadata = vec![];
    for blob in &block.blobs {
        blob_metadata.push(sugondat_nmt::BlobMetadata {
            namespace: blob.namespace,
            leaf: sugondat_nmt::NmtLeaf {
                extrinsic_index: blob.extrinsic_index,
                who: blob.sender,
                blob_hash: blob.sha2_hash(),
            },
        });
    }
    let mut tree = sugondat_nmt::tree_from_blobs(blob_metadata);
    let blob_proof = tree.proof(namespace);
    blob_proof
}
