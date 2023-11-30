use codec::{Decode, Encode};
use jsonrpsee::Methods;
use sugondat_shim_common_rollkit::{Blob, JsonRPCError, RollkitRPCServer};
use tracing::{debug, info};

use super::rpc_error as err;
use crate::{key::Keypair, sugondat_rpc};

/// Registers the sovereign dock in the given methods.
pub fn register(methods: &mut Methods, config: &super::Config) {
    debug!("enabling rollkit adapter dock");
    let dock = RollkitDock::new(config.client.clone(), config.submit_key.clone());
    methods
        .merge(dock.into_rpc())
        .expect("adapter namespace must be unique");
}

struct RollkitDock {
    client: sugondat_rpc::Client,
    submit_key: Option<Keypair>,
}

impl RollkitDock {
    fn new(client: sugondat_rpc::Client, submit_key: Option<Keypair>) -> Self {
        Self { client, submit_key }
    }
}

#[derive(codec::Encode, codec::Decode)]
pub struct Batch(Vec<Vec<u8>>);

impl From<Vec<Blob>> for Batch {
    fn from(value: Vec<Blob>) -> Self {
        Self(value.into_iter().map(|blob| blob.data).collect())
    }
}

#[async_trait::async_trait]
impl RollkitRPCServer for RollkitDock {
    async fn retrieve(&self, namespace: String, height: u64) -> Result<Vec<Blob>, JsonRPCError> {
        info!(
            "retrieving blobs from namespace '{}' at {}",
            namespace, height
        );
        let namespace = parse_namespace(&namespace).map_err(|_| err::bad_namespace())?;
        let block_hash = self.client.wait_finalized_height(height).await.unwrap();
        let block = self.client.get_block_at(block_hash).await.unwrap();
        let mut blobs = vec![];
        // From the sugondat perspective in the block are contained blobs
        // but each sugondat-blob is a rollkit-batch that could contain multiple rollkit-blobs
        for batch in block.blobs {
            if batch.namespace == namespace {
                let batch_data: Batch = Decode::decode(&mut &batch.data[..]).unwrap();
                blobs.extend(batch_data.0.into_iter().map(|blob| Blob { data: blob }));
            }
        }
        Ok(blobs)
    }

    async fn submit(&self, namespace: String, blobs: Vec<Blob>) -> Result<u64, JsonRPCError> {
        info!("submitting blob to namespace {}", namespace);
        let namespace = parse_namespace(&namespace).map_err(|_| err::bad_namespace())?;
        let submit_key = self
            .submit_key
            .as_ref()
            .cloned()
            .ok_or_else(err::no_signing_key)?;
        let batch: Batch = blobs.into();
        let block_hash = self
            .client
            .submit_blob(batch.encode(), namespace, submit_key.clone())
            .await
            .map_err(|_| err::submission_error())?;
        let block = self.client.get_block_at(block_hash).await.unwrap();
        Ok(block.number)
    }
}

/// Parses the namespace from a given string encoded as hex.
///
/// Note that rollkit uses arbitrary length namespaces, but sugondat uses 4-byte namespaces. For
/// now, we would silently truncate or pad the namespace to 4 bytes.
fn parse_namespace(namespace: &str) -> anyhow::Result<sugondat_nmt::Namespace> {
    let namespace_bytes = hex::decode(namespace)?;
    let namespace_bytes = match namespace_bytes.len() {
        0 => anyhow::bail!("namespace must not be empty"),
        1..=4 => {
            let mut namespace_bytes = namespace_bytes;
            namespace_bytes.resize(4, 0);
            namespace_bytes
        }
        5.. => {
            let mut namespace_bytes = namespace_bytes;
            namespace_bytes.truncate(4);
            namespace_bytes
        }
        _ => unreachable!(),
    };
    Ok(sugondat_nmt::Namespace::from_raw_bytes(
        namespace_bytes.try_into().unwrap(),
    ))
}
