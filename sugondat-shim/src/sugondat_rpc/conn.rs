//! Connection management module.

use super::FinalizedHeadWatcher;
use std::{
    mem,
    sync::{atomic::AtomicU64, Arc},
};
use subxt::backend::rpc::RpcClient;
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

/// Tries to find the `Blob` pallet in the runtime metadata. If it's not there, then we are not
/// connected to a Sugondat node.
fn check_if_compatible(client: &sugondat_subxt::Client) -> anyhow::Result<()> {
    assert!(sugondat_subxt::sugondat::PALLETS.contains(&"Blobs"));
    if let Some(pallet) = client.metadata().pallet_by_name("Blobs") {
        if pallet.call_variant_by_name("submit_blob").is_some() {
            return Ok(());
        }
    }
    Err(anyhow::anyhow!(
        "connected to a Substrate node that is not Sugondat"
    ))
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
                        tracing::error!(?conn_id, "failed to connect to sugondat node: {}\n", e);
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
