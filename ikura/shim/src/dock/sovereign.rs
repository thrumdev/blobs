use ikura_shim_common_sovereign::{Block, SovereignRPCServer};
use jsonrpsee::{server::Server, types::ErrorObjectOwned};
use tracing::info;

use super::rpc_error as err;
use crate::{ikura_rpc, key::Keypair};

pub struct Config {
    /// The RPC client handle to the ikura node.
    pub client: ikura_rpc::Client,

    /// The optional key used for signing when submitting blobs.
    pub submit_key: Option<Keypair>,

    /// The address to listen on.
    pub address: String,

    /// The port to listen on.
    pub port: u16,
}

/// Registers the sovereign dock in the given methods.
pub async fn run(config: Config) -> anyhow::Result<()> {
    let listen_on = (config.address.as_str(), config.port);
    let server = Server::builder().build(listen_on).await?;
    let dock = SovereignDock::new(config.client.clone(), config.submit_key.clone()).into_rpc();
    let handle = server.start(dock);
    handle.stopped().await;
    Ok(())
}

struct SovereignDock {
    client: ikura_rpc::Client,
    submit_key: Option<Keypair>,
}

impl SovereignDock {
    fn new(client: ikura_rpc::Client, submit_key: Option<Keypair>) -> Self {
        Self { client, submit_key }
    }
}

#[async_trait::async_trait]
impl SovereignRPCServer for SovereignDock {
    async fn get_block(
        &self,
        height: u64,
        namespace: ikura_nmt::Namespace,
    ) -> Result<Block, ErrorObjectOwned> {
        info!("get_block({})", height);
        let block_hash = self.client.await_finalized_height(height).await;
        let block = self.client.await_block_at(Some(block_hash)).await.unwrap();
        let proof = make_namespace_proof(&block, namespace);
        let blobs = block
            .blobs
            .into_iter()
            .filter(|blob| blob.namespace == namespace)
            .map(|blob| ikura_shim_common_sovereign::Blob {
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
        namespace: ikura_nmt::Namespace,
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
    block: &ikura_rpc::Block,
    namespace: ikura_nmt::Namespace,
) -> ikura_nmt::NamespaceProof {
    let mut blob_metadata = vec![];
    for blob in &block.blobs {
        blob_metadata.push(ikura_nmt::BlobMetadata {
            namespace: blob.namespace,
            leaf: ikura_nmt::NmtLeaf {
                extrinsic_index: blob.extrinsic_index,
                who: blob.sender,
                blob_hash: blob.sha2_hash(),
            },
        });
    }
    let mut tree = ikura_nmt::tree_from_blobs(blob_metadata);
    let blob_proof = tree.proof(namespace);
    blob_proof
}
