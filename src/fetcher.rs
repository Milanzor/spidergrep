use anyhow::{Context, Result};
use async_trait::async_trait;
use std::time::Duration;

pub struct PageContent {
    pub html: String,
    pub final_url: String,
}

#[async_trait]
pub trait Fetcher: Send + Sync {
    async fn fetch(&self, url: &str) -> Result<PageContent>;
}

pub struct HttpFetcher {
    client: reqwest::Client,
}

impl HttpFetcher {
    pub fn new(user_agent: &str, timeout_secs: u64, insecure: bool) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .user_agent(user_agent)
            .danger_accept_invalid_certs(insecure)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .context("Failed to build HTTP client")?;
        Ok(Self { client })
    }
}

#[async_trait]
impl Fetcher for HttpFetcher {
    async fn fetch(&self, url: &str) -> Result<PageContent> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;
        let final_url = resp.url().to_string();
        let html = resp.text().await.context("Reading response body")?;
        Ok(PageContent { html, final_url })
    }
}
