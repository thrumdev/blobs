//! A subxt client that is sync to initialize, but provides async interface.

use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Client {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    url: String,
    client: Option<sugondat_subxt::Client>,
}

impl Client {
    pub fn new(url: String) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner { url, client: None })),
        }
    }

    pub async fn client(&self) -> anyhow::Result<sugondat_subxt::Client> {
        let mut inner = self.inner.lock().await;
        if let Some(client) = &inner.client {
            return Ok(client.clone());
        }

        let client = sugondat_subxt::Client::from_url(&inner.url).await?;
        inner.client = Some(client.clone());
        Ok(client)
    }
}
