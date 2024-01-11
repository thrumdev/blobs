//! A subxt client that is sync to initialize, but provides async interface.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Client {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    url: String,
    request_timeout: Duration,
    client: Option<ClientRef>,
}

#[derive(Clone)]
pub struct ClientRef {
    client: Arc<jsonrpsee::ws_client::WsClient>,
}

impl std::ops::Deref for ClientRef {
    type Target = jsonrpsee::ws_client::WsClient;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl Client {
    pub fn new(url: String, request_timeout: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                url,
                request_timeout,
                client: None,
            })),
        }
    }

    pub async fn ensure_connected(&self) -> anyhow::Result<ClientRef> {
        let mut inner = self.inner.lock().await;
        if let Some(client) = &inner.client {
            return Ok(client.clone());
        }

        let client = jsonrpsee::ws_client::WsClientBuilder::new()
            .request_timeout(inner.request_timeout)
            .build(inner.url.clone())
            .await?;
        let client = ClientRef {
            client: Arc::new(client),
        };
        inner.client = Some(client.clone());
        Ok(client)
    }
}
