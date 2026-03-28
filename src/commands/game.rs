use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use futures::StreamExt;
use reqwest_eventsource::Event;

use crate::api::ApiClient;
use crate::auth;
use crate::config::Config;
use crate::crypto::{commit_hash, generate_nonce, sign_bytes};
use crate::keys;
use crate::output;
use crate::sse;
use crate::types::{
    ActionPayload, CommitPayload, EnqueueRequest, GameEvent, GameStatus, MatchEvent, RevealPayload,
};

pub async fn queue(api: &ApiClient, config: &Config, game: &str) -> Result<()> {
    let token = auth::ensure_token(api).await?;

    // Enqueue
    let req = EnqueueRequest {
        game_type: game.to_string(),
    };
    let _: serde_json::Value = api.post("/api/matching/enqueue", &req, &token).await?;

    // Wait for match via SSE
    let mut es = sse::connect(api, "/api/sse/matching", &token);
    let event: MatchEvent = sse::wait_for_event(&mut es).await?;
    es.close();

    match event {
        MatchEvent::MatchFound {
            game_id,
            opponent,
            game_type,
        } => {
            // Auto ready
            let _: serde_json::Value = api
                .post(
                    &format!("/api/game/{}/ready", game_id),
                    &serde_json::json!({}),
                    &token,
                )
                .await?;

            output::print_result(
                config,
                &serde_json::json!({
                    "gameId": game_id,
                    "opponent": opponent,
                    "gameType": game_type,
                }),
            )?;
        }
        MatchEvent::MatchTimeout => {
            anyhow::bail!("matchmaking timed out");
        }
    }
    Ok(())
}

pub async fn dequeue(api: &ApiClient, config: &Config) -> Result<()> {
    let token = auth::ensure_token(api).await?;
    let resp: serde_json::Value = api.delete("/api/matching/dequeue", &token).await?;
    output::print_raw(config, &resp)?;
    Ok(())
}

pub async fn status(api: &ApiClient, config: &Config, game_id: &str) -> Result<()> {
    let token = auth::ensure_token(api).await?;
    let resp: GameStatus = api.get(&format!("/api/game/{}", game_id), &token).await?;
    output::print_result(config, &resp)?;
    Ok(())
}

pub async fn submit(api: &ApiClient, config: &Config, game_id: &str, data: &str) -> Result<()> {
    let token = auth::ensure_token(api).await?;
    let (key, _) = keys::load_keys()?;

    // Parse data as JSON to validate
    let data_value: serde_json::Value =
        serde_json::from_str(data).map_err(|e| anyhow::anyhow!("invalid JSON data: {}", e))?;
    let data_str = serde_json::to_string(&data_value)?;

    // Get current game status to determine phase type
    let game_status: GameStatus = api.get(&format!("/api/game/{}", game_id), &token).await?;
    let phase = game_status
        .current_phase
        .ok_or_else(|| anyhow::anyhow!("no active phase"))?;

    // Start heartbeat
    let heartbeat = sse::spawn_heartbeat(api.clone(), game_id.to_string(), token.clone());

    let result = if phase.phase_type == "simultaneous" && phase.uses_commit_reveal {
        submit_cr(api, game_id, &phase.name, &key, &data_str, &data_value, &token).await
    } else {
        submit_action(api, game_id, &phase.name, &key, &data_value, &token).await
    };

    heartbeat.abort();

    let phase_result = result?;
    output::print_raw(config, &phase_result)?;
    Ok(())
}

async fn submit_cr(
    api: &ApiClient,
    game_id: &str,
    phase_id: &str,
    key: &ed25519_dalek::SigningKey,
    data_str: &str,
    data_value: &serde_json::Value,
    token: &str,
) -> Result<serde_json::Value> {
    // 1. Generate nonce and hash
    let nonce = generate_nonce();
    let hash = commit_hash(data_str, &nonce);
    let signature = sign_bytes(key, hash.as_bytes());
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // 2. Commit
    let commit = CommitPayload {
        game_id: game_id.to_string(),
        phase_id: phase_id.to_string(),
        hash,
        signature,
        timestamp,
    };
    let _: serde_json::Value = api
        .post(&format!("/api/game/{}/commit", game_id), &commit, token)
        .await?;

    // 3. Wait for all_committed via SSE
    let mut es = sse::connect(api, &format!("/api/sse/game/{}", game_id), token);
    loop {
        match es.next().await {
            Some(Ok(Event::Message(msg))) => {
                if let Ok(event) = serde_json::from_str::<GameEvent>(&msg.data) {
                    match event {
                        GameEvent::AllCommitted { .. } => break,
                        GameEvent::GameEnd { winner, reason } => {
                            es.close();
                            return Ok(serde_json::json!({
                                "type": "game_end",
                                "winner": winner,
                                "reason": reason,
                            }));
                        }
                        _ => {}
                    }
                }
            }
            Some(Ok(Event::Open)) => {}
            Some(Err(e)) => {
                es.close();
                anyhow::bail!("SSE error waiting for all_committed: {}", e);
            }
            None => {
                anyhow::bail!("SSE stream ended before all_committed");
            }
        }
    }

    // 4. Reveal
    let reveal_msg = format!("{}:{}", data_str, nonce);
    let reveal_sig = sign_bytes(key, reveal_msg.as_bytes());
    let reveal = RevealPayload {
        game_id: game_id.to_string(),
        phase_id: phase_id.to_string(),
        data: data_value.clone(),
        nonce,
        signature: reveal_sig,
    };
    let _: serde_json::Value = api
        .post(&format!("/api/game/{}/reveal", game_id), &reveal, token)
        .await?;

    // 5. Wait for phase_result
    loop {
        match es.next().await {
            Some(Ok(Event::Message(msg))) => {
                if let Ok(event) = serde_json::from_str::<GameEvent>(&msg.data) {
                    match event {
                        GameEvent::PhaseResult { result, .. } => {
                            es.close();
                            return Ok(result);
                        }
                        GameEvent::GameEnd { winner, reason } => {
                            es.close();
                            return Ok(serde_json::json!({
                                "type": "game_end",
                                "winner": winner,
                                "reason": reason,
                            }));
                        }
                        _ => {}
                    }
                }
            }
            Some(Ok(Event::Open)) => {}
            Some(Err(e)) => {
                es.close();
                anyhow::bail!("SSE error waiting for phase_result: {}", e);
            }
            None => {
                anyhow::bail!("SSE stream ended before phase_result");
            }
        }
    }
}

async fn submit_action(
    api: &ApiClient,
    game_id: &str,
    phase_id: &str,
    key: &ed25519_dalek::SigningKey,
    data_value: &serde_json::Value,
    token: &str,
) -> Result<serde_json::Value> {
    // 1. Sign and submit action
    let data_str = serde_json::to_string(data_value)?;
    let signature = sign_bytes(key, data_str.as_bytes());
    let action = ActionPayload {
        game_id: game_id.to_string(),
        phase_id: phase_id.to_string(),
        data: data_value.clone(),
        signature,
    };
    let _: serde_json::Value = api
        .post(&format!("/api/game/{}/action", game_id), &action, token)
        .await?;

    // 2. Wait for phase_result via SSE
    let mut es = sse::connect(api, &format!("/api/sse/game/{}", game_id), token);
    loop {
        match es.next().await {
            Some(Ok(Event::Message(msg))) => {
                if let Ok(event) = serde_json::from_str::<GameEvent>(&msg.data) {
                    match event {
                        GameEvent::PhaseResult { result, .. } => {
                            es.close();
                            return Ok(result);
                        }
                        GameEvent::GameEnd { winner, reason } => {
                            es.close();
                            return Ok(serde_json::json!({
                                "type": "game_end",
                                "winner": winner,
                                "reason": reason,
                            }));
                        }
                        _ => {}
                    }
                }
            }
            Some(Ok(Event::Open)) => {}
            Some(Err(e)) => {
                es.close();
                anyhow::bail!("SSE error waiting for phase_result: {}", e);
            }
            None => {
                anyhow::bail!("SSE stream ended before phase_result");
            }
        }
    }
}

pub async fn watch(api: &ApiClient, config: &Config, game_id: &str) -> Result<()> {
    let token = auth::ensure_token(api).await?;
    let mut es = sse::connect(api, &format!("/api/sse/spectate/{}", game_id), &token);

    while let Some(event) = es.next().await {
        match event {
            Ok(Event::Message(msg)) => {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&msg.data) {
                    output::print_raw(config, &value)?;
                    // Stop on game_end
                    if value.get("type").and_then(|t| t.as_str()) == Some("game_end") {
                        break;
                    }
                }
            }
            Ok(Event::Open) => {}
            Err(e) => {
                es.close();
                anyhow::bail!("SSE error: {}", e);
            }
        }
    }
    es.close();
    Ok(())
}
