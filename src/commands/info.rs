use anyhow::Result;

use crate::api::ApiClient;
use crate::auth;
use crate::config::Config;
use crate::output;
use crate::types::GameInfo;

pub async fn games(api: &ApiClient, config: &Config) -> Result<()> {
    let resp: Vec<GameInfo> = api.get_no_auth("/api/games").await?;
    output::print_result(config, &resp)?;
    Ok(())
}

pub async fn rules(api: &ApiClient, config: &Config, game: &str) -> Result<()> {
    let resp: serde_json::Value = api
        .get_no_auth(&format!("/api/games/{}/rules", game))
        .await?;
    output::print_raw(config, &resp)?;
    Ok(())
}

pub async fn rank(api: &ApiClient, config: &Config, target: &str) -> Result<()> {
    let token = auth::ensure_token(api).await?;
    let path = if target == "wealth" {
        "/api/leaderboard/wealth".to_string()
    } else {
        format!("/api/leaderboard/{}", target)
    };
    let resp: serde_json::Value = api.get_raw(&path, &token).await?;
    output::print_raw(config, &resp)?;
    Ok(())
}

pub async fn stats(api: &ApiClient, config: &Config, agent_id: Option<&str>) -> Result<()> {
    let token = auth::ensure_token(api).await?;
    let path = match agent_id {
        Some(id) => format!("/api/agent/{}/stats", id),
        None => {
            let cache = auth::load_token()?;
            format!("/api/agent/{}/stats", cache.agent_id)
        }
    };
    let resp: serde_json::Value = api.get_raw(&path, &token).await?;
    output::print_raw(config, &resp)?;
    Ok(())
}
