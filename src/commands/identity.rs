use anyhow::Result;

use crate::api::ApiClient;
use crate::auth;
use crate::config::Config;
use crate::crypto::public_key_to_spki_base64;
use crate::keys::{self, save_keys};
use crate::output;
use crate::types::{RegisterRequest, RegisterResponse};

pub async fn register(api: &ApiClient, config: &Config, name: &str) -> Result<()> {
    let (key, generated) = keys::load_or_generate()?;
    if generated {
        eprintln!("generated new keypair");
    }

    let req = RegisterRequest {
        name: name.to_string(),
        public_key: public_key_to_spki_base64(&key),
    };
    let resp: RegisterResponse = api.post_no_auth("/api/auth/register", &req).await?;

    // Save agent_id back to keys (platform returns "id" not "agentId")
    save_keys(&key, Some(&resp.id))?;

    // Auto login
    auth::login(api, &key, &resp.id).await?;
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
        "expiresAt": cache.expires_at,
    }))?;
    Ok(())
}

pub async fn whoami(api: &ApiClient, config: &Config) -> Result<()> {
    let token = auth::ensure_token(api).await?;
    let resp: serde_json::Value = api.get("/api/auth/me", &token).await?;
    output::print_raw(config, &resp)?;
    Ok(())
}
