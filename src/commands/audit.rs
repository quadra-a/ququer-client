use anyhow::Result;
use sha2::{Digest, Sha256};

use crate::api::ApiClient;
use crate::auth;
use crate::config::Config;
use crate::output;
use crate::types::SignedLogEntry;

pub async fn audit(api: &ApiClient, config: &Config, game_id: &str) -> Result<()> {
    let token = auth::ensure_token(api).await?;

    // Download log
    let log: Vec<SignedLogEntry> = api
        .get(&format!("/api/audit/{}/log", game_id), &token)
        .await?;

    // Verify hash chain
    let mut errors: Vec<String> = Vec::new();
    for i in 1..log.len() {
        let prev_entry_json = serde_json::to_string(&log[i - 1])?;
        let expected_hash = hex::encode(Sha256::digest(prev_entry_json.as_bytes()));
        if log[i].prev_hash != expected_hash {
            errors.push(format!("hash chain broken at seq {}", log[i].seq));
        }
    }

    let result = serde_json::json!({
        "gameId": game_id,
        "entries": log.len(),
        "valid": errors.is_empty(),
        "errors": errors,
    });
    output::print_raw(config, &result)?;
    Ok(())
}
