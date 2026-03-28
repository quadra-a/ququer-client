use anyhow::Result;
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::api::ApiClient;
use crate::config::ququer_dir;
use crate::crypto::sign_bytes;
use crate::keys;
use crate::types::{ChallengeResponse, LoginRequest, LoginResponse};

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenCache {
    pub token: String,
    pub expires_at: u64,
    pub agent_id: String,
}

fn token_path() -> Result<std::path::PathBuf> {
    Ok(ququer_dir()?.join("token.json"))
}

pub fn load_token() -> Result<TokenCache> {
    let path = token_path()?;
    let content = fs::read_to_string(&path)
        .map_err(|_| anyhow::anyhow!("not logged in — run `ququer login` first"))?;
    Ok(serde_json::from_str(&content)?)
}

fn save_token(cache: &TokenCache) -> Result<()> {
    let path = token_path()?;
    fs::write(&path, serde_json::to_string_pretty(cache)?)?;
    Ok(())
}

pub async fn login(api: &ApiClient, key: &SigningKey, agent_id: &str) -> Result<TokenCache> {
    // 1. Get challenge
    let challenge: ChallengeResponse = api
        .get_no_auth(&format!("/api/auth/challenge?agentId={}", agent_id))
        .await?;

    // 2. Sign challenge (platform expects base64 Ed25519 signature)
    let signature = sign_bytes(key, challenge.challenge.as_bytes());

    // 3. Login
    let req = LoginRequest {
        agent_id: agent_id.to_string(),
        challenge: challenge.challenge,
        signature,
    };
    let resp: LoginResponse = api.post_no_auth("/api/auth/login", &req).await?;

    // 4. Cache token
    let cache = TokenCache {
        token: resp.token,
        expires_at: resp.expires_at,
        agent_id: agent_id.to_string(),
    };
    save_token(&cache)?;
    Ok(cache)
}

pub async fn ensure_token(api: &ApiClient) -> Result<String> {
    let (key, stored) = keys::load_keys()?;
    let agent_id = stored
        .agent_id
        .ok_or_else(|| anyhow::anyhow!("no agent_id — run `ququer register` first"))?;

    match load_token() {
        Ok(cache) if !is_expired(cache.expires_at) => Ok(cache.token),
        _ => {
            let cache = login(api, &key, &agent_id).await?;
            Ok(cache.token)
        }
    }
}

fn is_expired(expires_at: u64) -> bool {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    now_ms >= expires_at
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_expired_future_timestamp() {
        assert!(!is_expired(u64::MAX));
    }

    #[test]
    fn is_expired_past_timestamp() {
        assert!(is_expired(0));
    }

    #[test]
    fn is_expired_one_second_ago() {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        assert!(is_expired(now_ms - 1000));
    }
}
