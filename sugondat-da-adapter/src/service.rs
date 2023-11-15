use crate::{
    spec::{ChainParams, DaLayerSpec},
    types,
    verifier::SugondatVerifier,
};
use async_trait::async_trait;
use sov_rollup_interface::{da::DaSpec, services::da::DaService};
use std::{future::Future, pin::Pin};
use subxt::backend::rpc::{rpc_params, RpcClient};
use sugondat_subxt::sugondat::{
    runtime_types::bounded_collections::bounded_vec::BoundedVec, storage, timestamp,
};

mod client;

use client::Client;

fn default_rpc_addr() -> String {
    "ws://localhost:9988/".into()
}

/// Runtime configuration for the DA service
#[derive(Default, Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DaServiceConfig {
    #[serde(default = "default_rpc_addr")]
    pub sugondat_rpc: String,
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
        let client = Client::new(config.sugondat_rpc);
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
        let client = self.client.clone();
        let namespace = self.namespace;
        let client_url = client.url().await;
        let client = client.client().await?;

        loop {
            let finalized_head = client.backend().latest_finalized_block_ref().await?;
            let header = client
                .backend()
                .block_header(finalized_head.hash())
                .await?
                .unwrap();
            if header.number as u64 >= height {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        // between version 0.29 and 0.32 they remove subxt::rpc::Rpc
        // so this 'raw' rpc call is required to extract the hash of the block with a certain height
        let rpc_client = RpcClient::from_url(client_url).await?;
        let hash: subxt::utils::H256 = rpc_client
            .request("chain_getBlockHash", rpc_params![height])
            .await?;

        let block = client.blocks().at(hash).await?;

        let header = block.header().clone();

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

        // fetch timestamp from block
        let timestamp = block
            .storage()
            .fetch(&storage().timestamp().now())
            .await?
            .ok_or(anyhow::anyhow!("no timestamp found"))?;

        let header = types::Header::new(
            types::Hash(hash.0),
            types::Hash(header.parent_hash.0),
            nmt_root.unwrap(),
            header.number as u64,
            timestamp,
        );

        let mut transactions = vec![];
        for ext in block.extrinsics().await?.iter() {
            let ext = ext?;
            let Some(address) = ext.address_bytes().map(|a| {
                tracing::info!("Address: {:?}", hex::encode(&a));
                types::Address::try_from(&a[1..]).unwrap()
            }) else {
                continue;
            };
            let Ok(Some(submit_blob_extrinsic)) =
                ext.as_extrinsic::<sugondat_subxt::sugondat::blob::calls::types::SubmitBlob>()
            else {
                // Not a submit blob extrinsic, skip.
                continue;
            };

            if submit_blob_extrinsic.namespace_id != namespace.namespace_id() {
                // Not for our app.
                continue;
            }

            let blob_data = submit_blob_extrinsic.blob.0;
            tracing::info!("received a blob: {}", hex::encode(&blob_data));
            transactions.push(types::BlobTransaction::new(address, blob_data));
        }

        let address = sugondat_subxt::sugondat::blob::storage::StorageApi.blob_list();
        let blobs = client
            .storage()
            .at(hash)
            .fetch(&address)
            .await
            .unwrap()
            .map(|x| x.0)
            .unwrap_or_default();

        let blobs = blobs
            .into_iter()
            .map(|blob| sugondat_nmt::BlobMetadata {
                namespace: sugondat_nmt::Namespace::with_namespace_id(blob.namespace_id),
                leaf: sugondat_nmt::NmtLeaf {
                    extrinsic_index: blob.extrinsic_index,
                    who: blob.who.0,
                    blob_hash: blob.blob_hash,
                },
            })
            .collect();
        let mut tree = sugondat_nmt::tree_from_blobs(blobs);
        let blob_proof = tree.proof(namespace);

        Ok(types::Block {
            header,
            transactions,
            blob_proof,
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
        blobs: &[<Self::Spec as DaSpec>::BlobTransaction],
    ) -> (
        <Self::Spec as DaSpec>::InclusionMultiProof,
        <Self::Spec as DaSpec>::CompletenessProof,
    ) {
        (block.blob_proof.clone(), ())
    }

    // Send the blob to the DA layer, using the submit_blob extrinsic
    async fn send_transaction(&self, blob: &[u8]) -> Result<(), Self::Error> {
        let client = self.client.clone();
        let blob = blob.to_vec();
        let namespace_id = self.namespace.namespace_id();
        use subxt_signer::sr25519::dev;

        let client = client.client().await?;

        let extrinsic = sugondat_subxt::sugondat::tx()
            .blob()
            .submit_blob(namespace_id, BoundedVec(blob));

        let from = dev::alice();
        let _events = client
            .tx()
            .sign_and_submit_then_watch_default(&extrinsic, &from)
            .await?
            .wait_for_finalized_success()
            .await?;

        Ok(())
    }
}
