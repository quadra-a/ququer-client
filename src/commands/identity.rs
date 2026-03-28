use anyhow::Result;

use crate::api::ApiClient;
use crate::auth;
use crate::config::Config;
use crate::keys::{self, public_key_hex, save_keys};
use crate::output;
use crate::types::{RegisterRequest, RegisterResponse};

pub async fn register(api: &ApiClient, config: &Config, name: &str) -> Result<()> {
    let (key, generated) = keys::load_or_generate()?;
    if generated {
        eprintln!("generated new keypair");
    }

    let req = RegisterRequest {
        name: name.to_string(),
        public_key: public_key_hex(&key),
    };
    let resp: RegisterResponse = api.post_no_auth("/api/auth/register", &req).await?;

    // Save agent_id back to keys
    save_keys(&key, Some(&resp.agent_id))?;

    // Auto login
    auth::login(api, &key, &resp.agent_id).await?;
    eprintln!("logged in");

    output::print_result(config, &resp)?;
    Ok(())
}

pub async fn login(api: &ApiClient, config: &Config) -> Result<()> {
    let (key, stored) = keys::load_keys()?;
    let agent_id = stored
        .agent_id
        .ok_or_else(|| anyhow::anyhow!("no agent_id — run `ququer register` first"))?;

    let cache = auth::login(api, &key, &agent_id).await?;
    output::print_result(config, &serde_json::json!({
        "agentId": cache.agent_id,
        "token": cache.token,
        "expiresAt": cache.expires_at,
    }))?;
    Ok(())
}

pub async fn whoami(_api: &ApiClient, config: &Config) -> Result<()> {
    let (_key, stored) = keys::load_keys()?;
    let token_info = auth::load_token().ok();

    output::print_result(config, &serde_json::json!({
        "publicKey": stored.public_key,
        "agentId": stored.agent_id,
        "loggedIn": token_info.is_some(),
    }))?;
    Ok(())
}
