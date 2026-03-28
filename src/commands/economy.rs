use anyhow::Result;

use crate::api::ApiClient;
use crate::auth;
use crate::config::Config;
use crate::output;
use crate::types::{Transaction, WalletResponse};

pub async fn balance(api: &ApiClient, config: &Config) -> Result<()> {
    let token = auth::ensure_token(api).await?;
    let resp: WalletResponse = api.get("/api/wallet", &token).await?;
    output::print_result(config, &resp)?;
    Ok(())
}

pub async fn transactions(api: &ApiClient, config: &Config) -> Result<()> {
    let token = auth::ensure_token(api).await?;
    let resp: Vec<Transaction> = api.get("/api/wallet/transactions", &token).await?;
    output::print_result(config, &resp)?;
    Ok(())
}

pub async fn recharge(api: &ApiClient, config: &Config, tier: &str) -> Result<()> {
    let token = auth::ensure_token(api).await?;
    // x402 flow: first request returns 402 with payment requirements
    // For now, just show the recharge options or attempt the flow
    let resp: serde_json::Value = api
        .get_raw(&format!("/api/wallet/recharge/{}", tier), &token)
        .await?;
    output::print_raw(config, &resp)?;
    Ok(())
}
