use ikura_shim_common_rollkit::{Blob, JsonRPCError, RollkitRPCServer};
use jsonrpsee::Methods;
use tracing::{debug, info};

use super::rpc_error as err;
use crate::{ikura_rpc, key::Keypair};

/// Registers the sovereign dock in the given methods.
pub fn register(methods: &mut Methods, config: &super::Config) {
    debug!("enabling rollkit adapter dock");
    let dock = RollkitDock::new(config.client.clone(), config.submit_key.clone());
    methods
        .merge(dock.into_rpc())
        .expect("adapter namespace must be unique");
}

struct RollkitDock {
    client: ikura_rpc::Client,
    submit_key: Option<Keypair>,
}

impl RollkitDock {
    fn new(client: ikura_rpc::Client, submit_key: Option<Keypair>) -> Self {
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
        let block_hash = self.client.await_finalized_height(height).await;
        let block = self.client.await_block_at(Some(block_hash)).await.unwrap();
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
                .map_err(err::submission_error)?;
        }
        // TODO:
        Ok(0)
    }
}

/// Parses the namespace from a given string encoded as hex.
///
/// Note that rollkit uses arbitrary length namespaces, but ikura uses 16-byte namespaces. For
/// now, we would silently truncate or pad the namespace to 16 bytes.
fn parse_namespace(namespace: &str) -> anyhow::Result<ikura_nmt::Namespace> {
    use ikura_nmt::NS_ID_SIZE;
    let namespace_bytes = hex::decode(namespace)?;
    if namespace_bytes.len() != NS_ID_SIZE {
        debug!(
            "The namespace '{}' is not {} bytes long. Resizing...",
            namespace, NS_ID_SIZE
        );
    }
    let namespace_bytes = match namespace_bytes.len() {
        0 => anyhow::bail!("namespace must not be empty"),
        1..=NS_ID_SIZE => {
            let mut namespace_bytes = namespace_bytes;
            namespace_bytes.resize(NS_ID_SIZE, 0);
            namespace_bytes
        }
        _ => {
            let mut namespace_bytes = namespace_bytes;
            namespace_bytes.truncate(NS_ID_SIZE);
            namespace_bytes
        }
    };
    Ok(ikura_nmt::Namespace::from_raw_bytes(
        namespace_bytes.try_into().unwrap(),
    ))
}

#[test]
fn test_parse_namespace() {
    use ikura_nmt::NS_ID_SIZE;
    assert!(parse_namespace("").is_err());
    assert!(parse_namespace("").is_err());
    assert_eq!(
        parse_namespace("00").unwrap(),
        ikura_nmt::Namespace::from_raw_bytes([0; NS_ID_SIZE])
    );
    assert_eq!(
        parse_namespace("00").unwrap(),
        ikura_nmt::Namespace::from_raw_bytes([0; NS_ID_SIZE])
    );
    assert_eq!(
        parse_namespace("FF").unwrap(),
        ikura_nmt::Namespace::from_raw_bytes({
            let mut bytes = [0; NS_ID_SIZE];
            bytes[0] = 0xFF;
            bytes
        })
    );
    assert_eq!(
        parse_namespace("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00" /* 17 bytes */).unwrap(),
        ikura_nmt::Namespace::from_raw_bytes([0xFF; NS_ID_SIZE])
    );
}
