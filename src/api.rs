use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Clone)]
pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub fn raw_client(&self) -> &reqwest::Client {
        &self.client
    }

    fn auth_headers(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
        );
        headers
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str, token: &str) -> Result<T> {
        let resp = self
            .client
            .get(self.url(path))
            .headers(Self::auth_headers(token))
            .send()
            .await
            .context("request failed")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {}: {}", status, body);
        }
        Ok(resp.json().await?)
    }

    pub async fn get_raw(&self, path: &str, token: &str) -> Result<serde_json::Value> {
        self.get(path, token).await
    }

    pub async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
        token: &str,
    ) -> Result<T> {
        let resp = self
            .client
            .post(self.url(path))
            .headers(Self::auth_headers(token))
            .json(body)
            .send()
            .await
            .context("request failed")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {}: {}", status, body);
        }
        Ok(resp.json().await?)
    }

    pub async fn post_no_auth<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let resp = self
            .client
            .post(self.url(path))
            .json(body)
            .send()
            .await
            .context("request failed")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {}: {}", status, body);
        }
        Ok(resp.json().await?)
    }

    pub async fn get_no_auth<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let resp = self
            .client
            .get(self.url(path))
            .send()
            .await
            .context("request failed")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {}: {}", status, body);
        }
        Ok(resp.json().await?)
    }

    pub async fn delete<T: DeserializeOwned>(&self, path: &str, token: &str) -> Result<T> {
        let resp = self
            .client
            .delete(self.url(path))
            .headers(Self::auth_headers(token))
            .send()
            .await
            .context("request failed")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {}: {}", status, body);
        }
        Ok(resp.json().await?)
    }
}
