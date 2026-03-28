use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use futures::StreamExt;
use reqwest_eventsource::Event;

use crate::api::{ApiClient, ApiError};
use crate::auth;
use crate::config::Config;
use crate::crypto::{commit_hash, generate_nonce, sign_bytes};
use crate::keys;
use crate::output;
use crate::sse;
use crate::types::{
    ActionPayload, ActiveGameResponse, CommitPayload, EnqueueRequest, GameEvent,
    GameStatusResponse, MatchEvent, PhaseInfo, RevealPayload,
};

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Extract current phase from visibleState
fn extract_phase(visible_state: &serde_json::Value) -> Result<PhaseInfo> {
    let phase_val = visible_state
        .get("currentPhase")
        .ok_or_else(|| anyhow::anyhow!("no active phase in visibleState"))?;
    let phase: PhaseInfo = serde_json::from_value(phase_val.clone())
        .map_err(|e| anyhow::anyhow!("failed to parse currentPhase: {}", e))?;
    Ok(phase)
}

pub async fn queue(api: &ApiClient, config: &Config, game: &str) -> Result<()> {
    let token = auth::ensure_token(api).await?;

    // Enqueue first — server clears stale SSE events on enqueue
    let req = EnqueueRequest {
        game_type: game.to_string(),
    };
    let _: serde_json::Value = match api.post("/api/matching/enqueue", &req, &token).await {
        Ok(v) => v,
        Err(e) => {
            if let Some(api_err) = e.downcast_ref::<ApiError>() {
                if api_err.status == 409 && api_err.body.contains("already_in_game") {
                    anyhow::bail!(
                        "You are already in an active game. \
                         Use `ququer active` to find your game ID, \
                         or `ququer forfeit <game_id>` to abandon it."
                    );
                }
            }
            return Err(e);
        }
    };

    // Connect SSE after enqueue — replay will pick up match_found if bot already matched
    let mut es = sse::connect(api, "/api/sse/matching", &token);

    // Wait for match via SSE
    let result = sse::wait_for_event::<MatchEvent>(&mut es).await;
    es.close();

    let event = match result {
        Ok(ev) => ev,
        Err(e) => {
            // Auto-dequeue on error so agent doesn't get stuck in already_enqueued state
            let _ = api.delete::<serde_json::Value>("/api/matching/dequeue", &token).await;
            return Err(e);
        }
    };

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
            // Server already removed us from queue on timeout, but dequeue defensively
            let _ = api.delete::<serde_json::Value>("/api/matching/dequeue", &token).await;
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

pub async fn active(api: &ApiClient, config: &Config) -> Result<()> {
    let token = auth::ensure_token(api).await?;
    let resp: ActiveGameResponse = api.get("/api/game/active", &token).await?;
    output::print_result(config, &resp)?;
    Ok(())
}

pub async fn forfeit(api: &ApiClient, config: &Config, game_id: &str) -> Result<()> {
    let token = auth::ensure_token(api).await?;
    let resp: serde_json::Value = api
        .post(
            &format!("/api/game/{}/forfeit", game_id),
            &serde_json::json!({}),
            &token,
        )
        .await?;
    output::print_raw(config, &resp)?;
    Ok(())
}

pub async fn status(api: &ApiClient, config: &Config, game_id: &str) -> Result<()> {
    let token = auth::ensure_token(api).await?;
    let resp: GameStatusResponse = api.get(&format!("/api/game/{}", game_id), &token).await?;
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

    // Start heartbeat early — covers the game status fetch and phase extraction
    let heartbeat = sse::spawn_heartbeat(api.clone(), game_id.to_string(), token.clone());

    // Get current game status to determine phase type
    let game_status: GameStatusResponse =
        api.get(&format!("/api/game/{}", game_id), &token).await?;
    let phase = extract_phase(&game_status.visible_state)?;

    let result = if phase.phase_type == "simultaneous" && phase.uses_commit_reveal {
        submit_cr(api, game_id, &phase, &key, &data_str, &data_value, &token).await
    } else {
        submit_action(api, game_id, &phase, &key, &data_value, &token).await
    };

    heartbeat.abort();

    let phase_result = result?;
    output::print_raw(config, &phase_result)?;
    Ok(())
}

async fn submit_cr(
    api: &ApiClient,
    game_id: &str,
    phase: &PhaseInfo,
    key: &ed25519_dalek::SigningKey,
    data_str: &str,
    data_value: &serde_json::Value,
    token: &str,
) -> Result<serde_json::Value> {
    // 1. Generate nonce and hash
    let nonce = generate_nonce();
    let hash = commit_hash(data_str, &nonce);
    let signature = sign_bytes(key, hash.as_bytes());

    // 2. Connect SSE first so we don't miss all_committed
    let mut es = sse::connect(api, &format!("/api/sse/game/{}", game_id), token);

    // 3. Commit
    let commit = CommitPayload {
        game_id: game_id.to_string(),
        phase_id: phase.id.clone(),
        hash,
        signature,
        timestamp: now_ms(),
    };
    let _: serde_json::Value = api
        .post(&format!("/api/game/{}/commit", game_id), &commit, token)
        .await?;

    // 4. Wait for all_committed via SSE (filter by current phase ID)
    loop {
        match es.next().await {
            Some(Ok(Event::Message(msg))) => {
                if let Ok(event) = serde_json::from_str::<GameEvent>(&msg.data) {
                    match event {
                        GameEvent::AllCommitted { phase: ref p } if *p == phase.id => break,
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
            Some(Err(_)) => {} // transient error, auto-retry
            None => {
                anyhow::bail!("SSE stream ended before all_committed");
            }
        }
    }

    // 5. Reveal
    let reveal_msg = format!("{}:{}", data_str, nonce);
    let reveal_sig = sign_bytes(key, reveal_msg.as_bytes());
    let reveal = RevealPayload {
        game_id: game_id.to_string(),
        phase_id: phase.id.clone(),
        data: data_value.clone(),
        nonce,
        signature: reveal_sig,
        timestamp: now_ms(),
    };
    let _: serde_json::Value = api
        .post(&format!("/api/game/{}/reveal", game_id), &reveal, token)
        .await?;

    // 6. Wait for phase_result (filter by current phase ID)
    loop {
        match es.next().await {
            Some(Ok(Event::Message(msg))) => {
                if let Ok(event) = serde_json::from_str::<GameEvent>(&msg.data) {
                    match event {
                        GameEvent::PhaseResult { result, phase: ref p } if *p == phase.id => {
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
            Some(Err(_)) => {} // transient error, auto-retry
            None => {
                anyhow::bail!("SSE stream ended before phase_result");
            }
        }
    }
}

async fn submit_action(
    api: &ApiClient,
    game_id: &str,
    phase: &PhaseInfo,
    key: &ed25519_dalek::SigningKey,
    data_value: &serde_json::Value,
    token: &str,
) -> Result<serde_json::Value> {
    // 1. Connect SSE first so we don't miss phase_result
    let mut es = sse::connect(api, &format!("/api/sse/game/{}", game_id), token);

    // 2. Sign and submit action
    let data_str = serde_json::to_string(data_value)?;
    let signature = sign_bytes(key, data_str.as_bytes());
    let action = ActionPayload {
        game_id: game_id.to_string(),
        phase_id: phase.id.clone(),
        action_type: phase.name.clone(),
        data: data_value.clone(),
        signature,
        timestamp: now_ms(),
    };
    let _: serde_json::Value = api
        .post(&format!("/api/game/{}/action", game_id), &action, token)
        .await?;

    // 3. Wait for phase_result via SSE
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
            Some(Err(_)) => {} // transient error, auto-retry
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
                    if value.get("type").and_then(|t| t.as_str()) == Some("game_end") {
                        break;
                    }
                }
            }
            Ok(Event::Open) => {}
            Err(_) => {} // transient error, auto-retry
        }
    }
    es.close();
    Ok(())
}
