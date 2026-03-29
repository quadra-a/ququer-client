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

pub async fn recharge(api: &ApiClient, config: &Config, amount: u64) -> Result<()> {
    let token = auth::ensure_token(api).await?;
    let body = serde_json::json!({ "amount": amount });
    let resp: serde_json::Value = api
        .post("/api/wallet/recharge", &body, &token)
        .await?;
    output::print_raw(config, &resp)?;
    Ok(())
}
