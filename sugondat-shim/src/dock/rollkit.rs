use jsonrpsee::Methods;
use sugondat_shim_common_rollkit::{Blob, JsonRPCError, RollkitRPCServer};
use tracing::{debug, info};

use crate::{key::Keypair, sugondat_rpc};

/// Registers the sovereign dock in the given methods.
pub fn register(methods: &mut Methods, config: &super::Config) {
    debug!("enabling rollkit adapter dock");
    let dock = RollkitDock::new(config.client.clone(), config.submit_key.clone());
    methods.merge(dock.into_rpc()).unwrap();
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

#[async_trait::async_trait]
impl RollkitRPCServer for RollkitDock {
    async fn retrieve(&self, namespace: String, height: u64) -> Result<Vec<Blob>, JsonRPCError> {
        info!(
            "retrieving blobs from namespace '{}' at {}",
            namespace, height
        );
        let namespace = parse_namespace(&namespace).map_err(|_| err::bad_namespace())?;
        let block_hash = self.client.wait_finalized_height(height).await;
        let block = self.client.get_block_at(block_hash).await.unwrap();
        let mut blobs = vec![];
        for blob in block.blobs {
            if blob.namespace == namespace {
                blobs.push(Blob { data: blob.data });
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
        for blob in blobs {
            self.client
                .submit_blob(blob.data, namespace, submit_key.clone())
                .await
                .unwrap();
        }
        // TODO:
        Ok(0)
    }
}

mod err {
    use sugondat_shim_common_rollkit::JsonRPCError;

    pub fn bad_namespace() -> JsonRPCError {
        JsonRPCError::owned(
            jsonrpsee::types::error::INVALID_PARAMS_CODE,
            "Invalid namespace",
            None::<()>,
        )
    }

    pub fn no_signing_key() -> JsonRPCError {
        JsonRPCError::owned(
            jsonrpsee::types::error::INTERNAL_ERROR_CODE,
            "Internal Error: no key for signing blobs",
            None::<()>,
        )
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
