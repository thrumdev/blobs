use std::sync::Arc;

use crate::key::Keypair;
use anyhow::Context;
use subxt::{backend::rpc::RpcClient, rpc_params, utils::H256};
use sugondat_nmt::Namespace;
use sugondat_subxt::{
    sugondat::runtime_types::bounded_collections::bounded_vec::BoundedVec, Header,
};
use tokio::sync::watch;
use tracing::Level;

// NOTE: we specifically avoid prolifiration of subxt types around the codebase. To that end, we
//       avoid returning H256 and instead return [u8; 32] directly.

mod conn {
    //! Connection management module.
    //!
    use std::{mem, sync::atomic::AtomicU64};

    use super::*;
    use sugondat_subxt::sugondat::is_codegen_valid_for;
    use tokio::sync::{oneshot, Mutex};

    // Contains the RPC client structures that are assumed to be connected.
    pub struct Conn {
        /// Connection id. For diagnostics purposes only.
        pub conn_id: u64,
        pub raw: RpcClient,
        pub subxt: sugondat_subxt::Client,
        pub finalized: FinalizedHeadWatcher,
    }

    impl Conn {
        async fn connect(conn_id: u64, rpc_url: &str) -> anyhow::Result<Arc<Self>> {
            let raw = RpcClient::from_url(rpc_url).await?;
            let subxt = sugondat_subxt::Client::from_rpc_client(raw.clone()).await?;
            check_if_compatible(&subxt)?;
            if !is_codegen_valid_for(&subxt.metadata()) {
                const WARN_WRONG_VERSION: &str = "connected to a sugondat node with a newer runtime than the one this shim was compiled against. Update the shim lest you run into problems. https://github.com/thrumdev/sugondat";
                tracing::warn!(WARN_WRONG_VERSION);
            }
            let finalized = FinalizedHeadWatcher::spawn(subxt.clone()).await;
            Ok(Arc::new(Self {
                conn_id,
                raw,
                subxt,
                finalized,
            }))
        }
    }

    enum State {
        /// The client is known to be connected.
        ///
        /// When the client experiences an error, there could be a brief state where the client is
        /// disconnected, but the connection has not been reset yet.
        Connected(Arc<Conn>),
        /// The client is currently connecting. The waiters are notified when the connection is
        /// established.
        Connecting {
            conn_id: u64,
            waiting: Vec<oneshot::Sender<Arc<Conn>>>,
        },
        /// Mostly used for as a dummy state during initialization, because the client should always
        /// be connected or connecting.
        Disconnected,
    }

    /// A struct that abstracts the connection concerns.
    ///
    /// Allows to wait for a connection to be established and to reset the connection if we detect
    /// that it's broken.
    pub struct Connector {
        state: Arc<Mutex<State>>,
        next_conn_id: AtomicU64,
        rpc_url: Arc<String>,
    }

    impl Connector {
        pub fn new(rpc_url: Arc<String>) -> Self {
            Self {
                state: Arc::new(Mutex::new(State::Disconnected)),
                next_conn_id: AtomicU64::new(0),
                rpc_url,
            }
        }

        /// Makes sure that the client is connected. Returns the connection handle.
        pub async fn ensure_connected(&self) -> Arc<Conn> {
            let mut state = self.state.lock().await;
            match &mut *state {
                State::Connected(conn) => {
                    let conn_id = conn.conn_id;
                    tracing::debug!(?conn_id, "reusing existing connection");
                    conn.clone()
                }
                State::Connecting {
                    conn_id,
                    ref mut waiting,
                } => {
                    // Somebody else is already connecting, let them cook.
                    tracing::debug!(?conn_id, "waiting for existing connection");
                    let (tx, rx) = oneshot::channel();
                    waiting.push(tx);
                    drop(state);

                    rx.await.expect("cannot be dropped")
                }
                State::Disconnected => {
                    // We are the first to connect.
                    //
                    // Important part: if the task performing the connection is cancelled,
                    // the `waiters` won't be notified and will wait forever unless we implement
                    // mitigation measures.
                    //
                    // Instead, we just spawn a new task here and the current task will wait
                    // similarly to the other waiters.

                    // Step 1: set the state to `Connecting` registering ourselves as a waiter.
                    let conn_id = self.gen_conn_id();
                    let (tx, rx) = oneshot::channel();
                    *state = State::Connecting {
                        conn_id,
                        waiting: vec![tx],
                    };

                    // Step 2: spawn the connection task.
                    self.spawn_connection_task(conn_id);
                    drop(state);

                    // Step 3: wait for the connection to be established.
                    rx.await.expect("cannot be dropped")
                }
            }
        }

        /// Drop the current connection and start a new connection task.
        pub async fn reset(&self) {
            let mut state = self.state.lock().await;
            match *state {
                State::Connecting { conn_id, .. } => {
                    // Guard against initiating a new connection when one is already in progress.
                    tracing::debug!(?conn_id, "reset: reconnection already in progress");
                    return;
                }
                State::Connected(ref conn) => {
                    let conn_id = conn.conn_id;
                    tracing::debug!(?conn_id, "reset: dropping connection");
                }
                State::Disconnected => (),
            }
            let conn_id = self.gen_conn_id();
            tracing::debug!(?conn_id, "reset: initiating new connection");
            *state = State::Connecting {
                conn_id,
                waiting: vec![],
            };
            self.spawn_connection_task(conn_id);
            drop(state);
        }

        fn gen_conn_id(&self) -> u64 {
            use std::sync::atomic::Ordering;
            let conn_id = self.next_conn_id.fetch_add(1, Ordering::Relaxed);
            conn_id
        }

        /// Spawns a task that will connect to the sugondat node and notify all waiters.
        fn spawn_connection_task(&self, conn_id: u64) {
            let state = self.state.clone();
            let rpc_url = self.rpc_url.clone();
            let _ = tokio::spawn(async move {
                tracing::debug!(?conn_id, ?rpc_url, "connecting to sugondat node");
                let conn = loop {
                    match Conn::connect(conn_id, &rpc_url).await {
                        Ok(conn) => break conn,
                        Err(e) => {
                            tracing::error!(
                                ?conn_id,
                                "failed to connect to sugondat node: {}\n",
                                e
                            );
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        }
                    }
                };

                let mut state = state.lock().await;
                let waiters = match &mut *state {
                    State::Connected(_) => {
                        // only one task is allowed to connect, and in this case it's us.
                        unreachable!()
                    }
                    State::Connecting {
                        conn_id: actual_conn_id,
                        ref mut waiting,
                    } => {
                        debug_assert_eq!(conn_id, *actual_conn_id);
                        mem::take(waiting)
                    }
                    State::Disconnected => {
                        debug_assert!(false, "unexpected state");
                        vec![]
                    }
                };

                // Finally, set the state to `Connected`, notify all waiters and explicitly
                // release the mutex.
                for tx in waiters {
                    let _ = tx.send(conn.clone());
                }
                *state = State::Connected(conn);
                drop(state);

                tracing::info!(?conn_id, "connected to sugondat node");
            });
        }
    }
}

/// A high-level abstraction over a sugondat RPC client.
///
/// This client abstracts over the connection concerns and will perform automatic reconnections in
/// case of network failures. This implies that all methods will retry indefinitely until they
/// succeed.
///
/// The assumption is that the sugondat node is not malicious and generally well-behaved.
///
/// # Clone
///
/// This is a thin wrapper that can be cloned cheaply.
#[derive(Clone)]
pub struct Client {
    connector: Arc<conn::Connector>,
}

impl Client {
    /// Creates a new instance of the client. This immediately tries to connect to the sugondat
    /// node. It will *retry indefinitely* until it succeeds.
    ///
    /// The RPC URL must be a valid URL pointing to a sugondat node. If it's not a malformed URL,
    /// returns an error.
    #[tracing::instrument(level = Level::DEBUG)]
    pub async fn new(rpc_url: String) -> anyhow::Result<Self> {
        anyhow::ensure!(
            url::Url::parse(&rpc_url).is_ok(),
            "invalid RPC URL: {}",
            rpc_url
        );

        tracing::info!("connecting to sugondat node: {}", rpc_url);
        let rpc_url = Arc::new(rpc_url);
        let me = Self {
            connector: Arc::new(conn::Connector::new(rpc_url)),
        };
        me.connector.ensure_connected().await;
        Ok(me)
    }

    /// Blocks until the sugondat node has finalized a block at the given height. Returns
    /// the block hash of the block at the given height.
    #[tracing::instrument(level = Level::DEBUG, skip(self))]
    pub async fn wait_finalized_height(&self, height: u64) -> [u8; 32] {
        loop {
            let conn = self.connector.ensure_connected().await;
            match conn.finalized.wait_until_finalized(self, height).await {
                Some(block_hash) => return block_hash,
                None => {
                    // The watcher task has terminated. Reset the connection and retry.
                    self.connector.reset().await;
                }
            }
        }
    }

    /// Returns the block hash of the block at the given height.
    ///
    /// If there is no block at the given height, returns `None`.
    #[tracing::instrument(level = Level::DEBUG, skip(self))]
    pub async fn block_hash(&self, height: u64) -> anyhow::Result<Option<H256>> {
        loop {
            let conn = self.connector.ensure_connected().await;
            let block_hash: Option<H256> = match conn
                .raw
                .request("chain_getBlockHash", rpc_params![height])
                .await
            {
                Ok(it) => it,
                Err(err) => {
                    tracing::error!(?err, "failed to query block hash");
                    self.connector.reset().await;
                    continue;
                }
            };

            break match block_hash {
                None => Ok(None),
                Some(h) if h == H256::zero() => {
                    // Little known fact: the sugondat node returns a zero block hash if there is no block
                    // at the given height.
                    Ok(None)
                }
                Some(block_hash) => Ok(Some(block_hash)),
            };
        }
    }

    /// Returns the header and the body of the block with the given hash, automatically retrying
    /// until it succeeds.
    async fn get_header_and_extrinsics(
        &self,
        block_hash: [u8; 32],
    ) -> anyhow::Result<(Header, Vec<sugondat_subxt::ExtrinsicDetails>)> {
        let block_hash = H256::from(block_hash);
        loop {
            let conn = self.connector.ensure_connected().await;
            let err = match conn.subxt.blocks().at(block_hash).await {
                Ok(it) => {
                    let header = it.header();
                    let body = match it.extrinsics().await {
                        Ok(it) => it,
                        Err(err) => {
                            tracing::error!(?err, "failed to query block");
                            self.connector.reset().await;
                            continue;
                        }
                    }
                    .iter()
                    .collect::<Result<Vec<_>, _>>()?;
                    return Ok((header.clone(), body));
                }
                Err(err) => err,
            };
            tracing::error!(?err, "failed to query block");
            self.connector.reset().await;
        }
    }

    /// Returns the data of the block identified by the given block hash. If the block is not found
    /// returns an error.
    #[tracing::instrument(level = Level::DEBUG, skip(self))]
    pub async fn get_block_at(&self, block_hash: [u8; 32]) -> anyhow::Result<Block> {
        let (header, extrinsics) = self.get_header_and_extrinsics(block_hash).await?;
        let tree_root = tree_root(&header).ok_or_else(err::no_tree_root)?;
        let timestamp = extract_timestamp(&extrinsics)?;
        let blobs = extract_blobs(extrinsics);
        tracing::debug!(?blobs, "found {} blobs in block", blobs.len());
        Ok(Block {
            number: header.number as u64,
            parent_hash: header.parent_hash.0,
            tree_root,
            timestamp,
            blobs,
        })
    }

    /// Submit a blob with the given namespace and signed with the given key. The block is submitted
    /// at best effort. Not much is done to ensure that the blob is actually included. If this
    /// function returned an error, that does not mean that the blob was not included.
    ///
    /// Returns a block hash in which the extrinsic was included.
    #[tracing::instrument(level = Level::DEBUG, skip(self))]
    pub async fn submit_blob(
        &self,
        blob: Vec<u8>,
        namespace: sugondat_nmt::Namespace,
        key: Keypair,
    ) -> anyhow::Result<[u8; 32]> {
        let namespace_id = namespace.to_u32_be();
        let extrinsic = sugondat_subxt::sugondat::tx()
            .blob()
            .submit_blob(namespace_id, BoundedVec(blob));

        let conn = self.connector.ensure_connected().await;
        let signed = conn
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

/// Iterates over the extrinsics in a block and extracts the timestamp.
///
/// The timestamp is considered a mandatory inherent extrinsic, therefore every block must have
/// one. An `Err` is returned if no timestamp is found.
fn extract_timestamp(extrinsics: &[sugondat_subxt::ExtrinsicDetails]) -> anyhow::Result<u64> {
    use sugondat_subxt::sugondat::timestamp::calls::types::Set as TimestampSet;
    for e in extrinsics.iter() {
        match e.as_extrinsic::<TimestampSet>() {
            Ok(Some(timestamp_extrinsic)) => return Ok(timestamp_extrinsic.now),
            _ => (),
        }
    }
    Err(anyhow::anyhow!("no timestamp found in block"))
}

/// Iterates over the extrinsics in a block and extracts the submit_blob extrinsics.
fn extract_blobs(extrinsics: Vec<sugondat_subxt::ExtrinsicDetails>) -> Vec<Blob> {
    use sugondat_subxt::sugondat::blob::calls::types::SubmitBlob;

    let mut blobs = vec![];
    for (extrinsic_index, e) in extrinsics.iter().enumerate() {
        let Some(sender) = e
            .address_bytes()
            .filter(|a| a.len() == 33)
            .and_then(|a| a[1..].try_into().ok())
        else {
            continue;
        };
        let Ok(Some(SubmitBlob { namespace_id, blob })) = e.as_extrinsic::<SubmitBlob>() else {
            // Not a submit blob extrinsic, skip.
            continue;
        };
        blobs.push(Blob {
            extrinsic_index: extrinsic_index as u32,
            namespace: sugondat_nmt::Namespace::from_u32_be(namespace_id),
            sender,
            data: blob.0,
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
                // In case of an error, the subxt client becomes unusable. The task will be
                // terminated in case of an error.
                let Ok(mut stream) = subxt.backend().stream_finalized_block_headers().await else {
                    return;
                };
                while let Some(header) = stream.next().await {
                    let Ok((header, block_ref)) = header else {
                        return;
                    };
                    let _ = tx.send((header.number as u64, block_ref.hash().0));
                }
            }
        });
        Self { rx, handle }
    }

    /// Wait until the sugondat node has finalized a block at the given height. Returns the block
    /// hash of that finalized block, or `None` in case the watcher task has terminated.
    async fn wait_until_finalized(&self, client: &Client, height: u64) -> Option<[u8; 32]> {
        let mut rx = self.rx.clone();
        let (finalized_height, block_hash) = loop {
            if let Err(_) = rx.changed().await {
                // The sender half was dropped means the watcher task has terminated.
                return None;
            }
            let (finalized_height, block_hash) = *rx.borrow();
            if finalized_height < height {
                continue;
            }
            break (finalized_height, block_hash);
        };
        if finalized_height == height {
            // The common case: the finalized block is at the height we're looking for.
            Some(block_hash)
        } else {
            // The finalized block is already past the height we're looking for, but we need to
            // return the block hash of the block at the given height. Therefore, we query it.
            loop {
                let Ok(Some(block_hash)) = client.block_hash(height).await else {
                    // TODO: throttle the retries. // TODO: return an error
                    continue;
                };
                break Some(block_hash.0);
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
