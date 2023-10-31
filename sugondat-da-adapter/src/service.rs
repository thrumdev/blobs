use std::{future::Future, pin::Pin};

use crate::spec::{ChainParams, DaLayerSpec};
use crate::types;

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
pub struct DaProvider {
    namespace: sugondat_nmt::Namespace,
    client: Client,
}

impl sov_rollup_interface::services::da::DaService for DaProvider {
    type Spec = DaLayerSpec;
    type Future<T> = Pin<Box<dyn Future<Output = Result<T, Self::Error>> + Send>>;
    type FilteredBlock = crate::types::Block;
    type Error = anyhow::Error;
    type RuntimeConfig = DaServiceConfig;

    /// Creates new instance of the service.
    fn new(config: DaServiceConfig, chain_params: ChainParams) -> Self {
        let client = Client::new(config.sugondat_rpc);
        Self {
            namespace: sugondat_nmt::Namespace::from_raw_bytes(chain_params.namespace_id),
            client,
        }
    }

    // Make an RPC call to the node to get the finalized block at the given height, if one exists.
    // If no such block exists, block until one does.
    fn get_finalized_at(&self, height: u64) -> Self::Future<Self::FilteredBlock> {
        let client = self.client.clone();
        let namespace = self.namespace;
        Box::pin(async move {
            let client = client.client().await?;

            loop {
                let finalized_head = client.rpc().finalized_head().await?;
                let header = client.rpc().header(Some(finalized_head)).await?.unwrap();
                if header.number as u64 >= height {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }

            let hash = client.rpc().block_hash(Some(height.into())).await?.unwrap();

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
            let header = types::Header::new(
                types::Hash(hash.0),
                types::Hash(header.parent_hash.0),
                nmt_root.unwrap(),
            );

            let body = block.body().await?;
            let mut transactions = vec![];
            for ext in body.extrinsics().iter() {
                let ext = ext?;
                let Some(address) = ext
                    .address_bytes()
                    .map(|a| {
                        tracing::info!("Address: {:?}", hex::encode(&a));
                        types::Address::try_from(&a[1..]).unwrap()
                    })
                else {
                    continue
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

            let address =
                sugondat_subxt::sugondat::blob::storage::StorageApi.blob_list();
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
        })
    }

    // Make an RPC call to the node to get the block at the given height
    // If no such block exists, block until one does.
    fn get_block_at(&self, height: u64) -> Self::Future<Self::FilteredBlock> {
        self.get_finalized_at(height)
    }

    // Extract the blob transactions relevant to a particular rollup from a block.
    // This method is usually (but not always) parameterized by some configuration option,
    // such as the rollup's namespace on Celestia. If configuration is needed, it should be provided
    // to the DaProvider struct through its constructor.
    fn extract_relevant_txs(
        &self,
        block: &Self::FilteredBlock,
    ) -> Vec<<Self::Spec as sov_rollup_interface::da::DaSpec>::BlobTransaction> {
        block.transactions.clone()
    }

    // Extract the list blob transactions relevant to a particular rollup from a block, along with inclusion and
    // completeness proofs for that set of transactions. The output of this method will be passed to the verifier.
    //
    // Like extract_relevant_txs, This method is usually (but not always) parameterized by some configuration option,
    // such as the rollup's namespace on Celestia. If configuration is needed, it should be provided
    // to the DaProvider struct through its constructor.
    fn extract_relevant_txs_with_proof(
        &self,
        block: &Self::FilteredBlock,
    ) -> (
        Vec<<Self::Spec as sov_rollup_interface::da::DaSpec>::BlobTransaction>,
        <Self::Spec as sov_rollup_interface::da::DaSpec>::InclusionMultiProof,
        <Self::Spec as sov_rollup_interface::da::DaSpec>::CompletenessProof,
    ) {
        let txs = self.extract_relevant_txs(block);
        let (inclusion_proof, completeness_proof) = self.get_extraction_proof(block, &txs);
        (txs, inclusion_proof, completeness_proof)
    }

    fn get_extraction_proof(
        &self,
        block: &Self::FilteredBlock,
        _blobs: &[<Self::Spec as sov_rollup_interface::da::DaSpec>::BlobTransaction],
    ) -> (
        <Self::Spec as sov_rollup_interface::da::DaSpec>::InclusionMultiProof,
        <Self::Spec as sov_rollup_interface::da::DaSpec>::CompletenessProof,
    ) {
        (block.blob_proof.clone(), ())
    }

    fn send_transaction(&self, blob: &[u8]) -> Self::Future<()> {
        drop(blob);
        // TODO: rework this
        // let client = self.client.clone();
        // let blob = blob.to_vec();
        // let namespace_id = self.chain_params.namespace_id;
        // Box::pin(async move {
        //     use sp_keyring::AccountKeyring;
        //     use subxt::tx::PairSigner;

        //     let client = client.client().await?;

        //     let mut raw = vec![];
        //     raw.extend_from_slice(namespace_id.as_ref());
        //     raw.extend_from_slice(&blob);

        //     let extrinsic = sugondat_subxt::sugondat::tx().system().remark(raw);
        //     let from = PairSigner::new(AccountKeyring::Alice.pair());
        //     let _events = client
        //         .tx()
        //         .sign_and_submit_then_watch_default(&extrinsic, &from)
        //         .await?
        //         .wait_for_finalized_success()
        //         .await?;

        //     Ok(())
        // })
        todo!()
    }
}
