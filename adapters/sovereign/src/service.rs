use crate::{
    spec::{ChainParams, DaLayerSpec},
    types::{self, Hash},
    verifier::SugondatVerifier,
};
use async_trait::async_trait;
use sov_rollup_interface::da::DaSpec;
use std::time::Duration;
use sugondat_shim_common_sovereign::SovereignRPCClient;

mod client;

use client::Client;

fn default_rpc_addr() -> String {
    "ws://localhost:10995/".into()
}

fn default_rpc_timeout_seconds() -> u64 {
    60
}

/// Runtime configuration for the DA service
#[derive(Default, Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DaServiceConfig {
    #[serde(default = "default_rpc_addr")]
    pub sugondat_rpc: String,
    #[serde(default = "default_rpc_timeout_seconds")]
    pub rpc_timeout_seconds: u64,
}

/// Implementation of the DA provider that uses sugondat.
#[derive(Clone)]
pub struct DaProvider {
    namespace: sugondat_nmt::Namespace,
    client: Client,
}

impl DaProvider {
    /// Creates new instance of the service.
    pub fn new(config: DaServiceConfig, chain_params: ChainParams) -> Self {
        let request_timeout = Duration::from_secs(config.rpc_timeout_seconds);
        let client = Client::new(config.sugondat_rpc, request_timeout);
        Self {
            namespace: sugondat_nmt::Namespace::from_raw_bytes(chain_params.namespace_id),
            client,
        }
    }
}

#[async_trait]
impl sov_rollup_interface::services::da::DaService for DaProvider {
    type Spec = DaLayerSpec;
    type FilteredBlock = crate::types::Block;
    type Error = anyhow::Error;
    type Verifier = SugondatVerifier;

    // Make an RPC call to the node to get the finalized block at the given height, if one exists.
    // If no such block exists, block until one does.
    async fn get_finalized_at(&self, height: u64) -> Result<Self::FilteredBlock, Self::Error> {
        let client = self.client.ensure_connected().await?;
        let block: sugondat_shim_common_sovereign::Block =
            client.get_block(height, self.namespace).await?;
        let header = types::Header::new(
            Hash(block.block_hash),
            Hash(block.prev_hash),
            block.nmt_root,
            height,
            block.timestamp,
        );
        let transactions = block
            .blobs
            .into_iter()
            .map(|blob| types::BlobTransaction::new(types::Address(blob.sender), blob.data))
            .collect();
        Ok(types::Block {
            header,
            transactions,
            blob_proof: block.proof,
        })
    }

    // Make an RPC call to the node to get the block at the given height
    // If no such block exists, block until one does.
    async fn get_block_at(&self, height: u64) -> Result<Self::FilteredBlock, Self::Error> {
        self.get_finalized_at(height).await
    }

    // Extract the blob transactions relevant to a particular rollup from a block.
    // This method is usually (but not always) parameterized by some configuration option,
    // such as the rollup's namespace on Celestia. If configuration is needed, it should be provided
    // to the DaProvider struct through its constructor.
    fn extract_relevant_blobs(
        &self,
        block: &Self::FilteredBlock,
    ) -> Vec<<Self::Spec as DaSpec>::BlobTransaction> {
        block.transactions.clone()
    }

    async fn get_extraction_proof(
        &self,
        block: &Self::FilteredBlock,
        _blobs: &[<Self::Spec as DaSpec>::BlobTransaction],
    ) -> (
        <Self::Spec as DaSpec>::InclusionMultiProof,
        <Self::Spec as DaSpec>::CompletenessProof,
    ) {
        (block.blob_proof.clone(), ())
    }

    // Send the blob to the DA layer, using the submit_blob extrinsic
    async fn send_transaction(&self, blob: &[u8]) -> Result<(), Self::Error> {
        let client = self.client.ensure_connected().await?;
        client.submit_blob(blob.to_vec(), self.namespace).await?;
        Ok(())
    }
}
