//! HTTP client for the pending prompt inspection API.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingItemResponse {
    pub message_id: Uuid,
    pub kind: String,
    pub channel: String,
    pub position: usize,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingListResponse {
    pub items: Vec<PendingItemResponse>,
    pub total_count: usize,
}

pub struct PendingClient {
    base_url: String,
    client: reqwest::Client,
}

impl PendingClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let base = base_url.into();
        let base = base.trim_end_matches('/').to_string();
        Self {
            base_url: base,
            client: reqwest::Client::new(),
        }
    }

    pub async fn list_pending(&self, channel: Option<&str>) -> anyhow::Result<PendingListResponse> {
        let mut url = format!("{}/api/v1/pending", self.base_url);
        if let Some(ch) = channel {
            url.push_str(&format!("?channel={}", ch));
        }
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("Server returned {}", resp.status());
        }
        Ok(resp.json::<PendingListResponse>().await?)
    }
}
