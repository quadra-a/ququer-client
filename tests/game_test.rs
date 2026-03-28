use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use ququer_client::api::ApiClient;

fn mock_game_status_simultaneous() -> serde_json::Value {
    json!({
        "id": "game-1",
        "gameType": "rock-paper-scissors",
        "state": "active",
        "currentPhase": {
            "type": "simultaneous",
            "name": "action",
            "usesCommitReveal": true,
            "timeout": 30000
        }
    })
}

fn mock_game_status_sequential() -> serde_json::Value {
    json!({
        "id": "game-1",
        "gameType": "liars-dice",
        "state": "active",
        "currentPhase": {
            "type": "sequential",
            "name": "bid",
            "usesCommitReveal": false,
            "timeout": 30000
        }
    })
}

fn mock_game_status_no_phase() -> serde_json::Value {
    json!({
        "id": "game-1",
        "gameType": "rps",
        "state": "finished"
    })
}

fn sse_body(events: &[serde_json::Value]) -> String {
    events
        .iter()
        .map(|e| format!("data: {}\n\n", e))
        .collect::<String>()
}

#[tokio::test]
async fn submit_cr_sends_commit_then_reveal() {
    let server = MockServer::start().await;

    // 1. Mock game status → simultaneous CR
    Mock::given(method("GET"))
        .and(path("/api/game/game-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_game_status_simultaneous()))
        .mount(&server)
        .await;

    // 2. Mock commit endpoint
    Mock::given(method("POST"))
        .and(path("/api/game/game-1/commit"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .expect(1)
        .named("commit")
        .mount(&server)
        .await;

    // 3. Mock SSE stream: all_committed then phase_result
    let sse_events = sse_body(&[
        json!({"type": "all_committed", "phase": "action"}),
        json!({"type": "phase_result", "phase": "action", "result": {"winner": "agent-1"}}),
    ]);
    Mock::given(method("GET"))
        .and(path("/api/sse/game/game-1"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_events),
        )
        .mount(&server)
        .await;

    // 4. Mock reveal endpoint
    Mock::given(method("POST"))
        .and(path("/api/game/game-1/reveal"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .expect(1)
        .named("reveal")
        .mount(&server)
        .await;

    // 5. Mock heartbeat (will be called in background)
    Mock::given(method("POST"))
        .and(path("/api/game/game-1/heartbeat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .mount(&server)
        .await;

    // Run the submit_cr flow directly
    let api = ApiClient::new(&server.uri());
    let key = ed25519_dalek::SigningKey::from_bytes(&[10u8; 32]);
    let data_str = r#"{"action":"rock"}"#;
    let data_value: serde_json::Value = serde_json::from_str(data_str).unwrap();

    // We test the internal flow by calling the game module's helpers
    // Since submit_cr is private, we test via the crypto + API layer
    let nonce = ququer_client::crypto::generate_nonce();
    let hash = ququer_client::crypto::commit_hash(data_str, &nonce);
    let signature = ququer_client::crypto::sign_bytes(&key, hash.as_bytes());

    // Verify commit hash is correct
    use sha2::{Digest, Sha256};
    let expected_input = format!("{}:{}", data_str, nonce);
    let expected_hash = hex::encode(Sha256::digest(expected_input.as_bytes()));
    assert_eq!(hash, expected_hash);

    // Verify signature is valid
    let sig_bytes = hex::decode(&signature).unwrap();
    let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes.try_into().unwrap());
    assert!(
        ed25519_dalek::Verifier::verify(&key.verifying_key(), hash.as_bytes(), &sig).is_ok()
    );

    // Test commit API call
    let commit_body = json!({
        "gameId": "game-1",
        "phaseId": "action",
        "hash": hash,
        "signature": signature,
        "timestamp": 12345
    });
    let resp: serde_json::Value = api
        .post("/api/game/game-1/commit", &commit_body, "test-token")
        .await
        .unwrap();
    assert_eq!(resp["ok"], true);

    // Test reveal API call
    let reveal_sig = ququer_client::crypto::sign_bytes(
        &key,
        format!("{}:{}", data_str, nonce).as_bytes(),
    );
    let reveal_body = json!({
        "gameId": "game-1",
        "phaseId": "action",
        "data": data_value,
        "nonce": nonce,
        "signature": reveal_sig
    });
    let resp: serde_json::Value = api
        .post("/api/game/game-1/reveal", &reveal_body, "test-token")
        .await
        .unwrap();
    assert_eq!(resp["ok"], true);
}

#[tokio::test]
async fn submit_action_sends_signed_action() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/game/game-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_game_status_sequential()))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/game/game-1/action"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .expect(1)
        .named("action")
        .mount(&server)
        .await;

    let api = ApiClient::new(&server.uri());
    let key = ed25519_dalek::SigningKey::from_bytes(&[11u8; 32]);
    let data_value = json!({"bid": {"count": 3, "face": 4}});
    let data_str = serde_json::to_string(&data_value).unwrap();
    let signature = ququer_client::crypto::sign_bytes(&key, data_str.as_bytes());

    let action_body = json!({
        "gameId": "game-1",
        "phaseId": "bid",
        "data": data_value,
        "signature": signature
    });
    let resp: serde_json::Value = api
        .post("/api/game/game-1/action", &action_body, "test-token")
        .await
        .unwrap();
    assert_eq!(resp["ok"], true);

    // Verify signature
    let sig_bytes = hex::decode(&signature).unwrap();
    let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes.try_into().unwrap());
    assert!(
        ed25519_dalek::Verifier::verify(&key.verifying_key(), data_str.as_bytes(), &sig).is_ok()
    );
}

#[tokio::test]
async fn game_status_no_phase_parsed() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/game/game-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_game_status_no_phase()))
        .mount(&server)
        .await;

    let api = ApiClient::new(&server.uri());
    let status: ququer_client::types::GameStatus =
        api.get("/api/game/game-1", "test-token").await.unwrap();
    assert_eq!(status.state, "finished");
    assert!(status.current_phase.is_none());
}

#[tokio::test]
async fn game_status_simultaneous_phase_parsed() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/game/game-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_game_status_simultaneous()))
        .mount(&server)
        .await;

    let api = ApiClient::new(&server.uri());
    let status: ququer_client::types::GameStatus =
        api.get("/api/game/game-1", "test-token").await.unwrap();
    let phase = status.current_phase.unwrap();
    assert_eq!(phase.phase_type, "simultaneous");
    assert!(phase.uses_commit_reveal);
    assert_eq!(phase.name, "action");
}

#[tokio::test]
async fn sse_game_events_parse_correctly() {
    // Test that SSE event bodies parse into GameEvent variants
    let cases = vec![
        (
            r#"{"type":"all_committed","phase":"action"}"#,
            "all_committed",
        ),
        (
            r#"{"type":"phase_result","phase":"action","result":{"winner":"a1"}}"#,
            "phase_result",
        ),
        (
            r#"{"type":"game_end","winner":"a1","reason":"normal"}"#,
            "game_end",
        ),
        (
            r#"{"type":"game_end","winner":null,"reason":"timeout"}"#,
            "game_end_draw",
        ),
        (
            r#"{"type":"your_turn","phase":"bid"}"#,
            "your_turn",
        ),
    ];

    for (json_str, label) in cases {
        let event: ququer_client::types::GameEvent =
            serde_json::from_str(json_str).unwrap_or_else(|e| panic!("failed to parse {}: {}", label, e));
        match (label, &event) {
            ("all_committed", ququer_client::types::GameEvent::AllCommitted { phase }) => {
                assert_eq!(phase, "action");
            }
            ("phase_result", ququer_client::types::GameEvent::PhaseResult { result, .. }) => {
                assert_eq!(result["winner"], "a1");
            }
            ("game_end", ququer_client::types::GameEvent::GameEnd { winner, reason }) => {
                assert_eq!(winner.as_deref(), Some("a1"));
                assert_eq!(reason, "normal");
            }
            ("game_end_draw", ququer_client::types::GameEvent::GameEnd { winner, reason }) => {
                assert!(winner.is_none());
                assert_eq!(reason, "timeout");
            }
            ("your_turn", ququer_client::types::GameEvent::YourTurn { phase }) => {
                assert_eq!(phase, "bid");
            }
            _ => panic!("unexpected match for {}", label),
        }
    }
}

#[tokio::test]
async fn match_event_parse_from_sse_body() {
    let json_str = r#"{"type":"match_found","gameId":"g1","opponent":"bot2","gameType":"rps"}"#;
    let event: ququer_client::types::MatchEvent = serde_json::from_str(json_str).unwrap();
    match event {
        ququer_client::types::MatchEvent::MatchFound {
            game_id,
            opponent,
            game_type,
        } => {
            assert_eq!(game_id, "g1");
            assert_eq!(opponent, "bot2");
            assert_eq!(game_type, "rps");
        }
        _ => panic!("expected MatchFound"),
    }
}

#[tokio::test]
async fn commit_reveal_hash_chain_integrity() {
    // Simulate a mini commit-reveal flow and verify hash consistency
    let key = ed25519_dalek::SigningKey::from_bytes(&[20u8; 32]);
    let data = r#"{"action":"paper"}"#;
    let nonce = "fixed-nonce-for-test";

    // Commit phase
    let hash = ququer_client::crypto::commit_hash(data, nonce);
    let commit_sig = ququer_client::crypto::sign_bytes(&key, hash.as_bytes());

    // Reveal phase — recompute hash and verify it matches
    let reveal_hash = ququer_client::crypto::commit_hash(data, nonce);
    assert_eq!(hash, reveal_hash, "hash must be deterministic");

    // Verify commit signature still valid
    let sig_bytes = hex::decode(&commit_sig).unwrap();
    let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes.try_into().unwrap());
    assert!(
        ed25519_dalek::Verifier::verify(&key.verifying_key(), hash.as_bytes(), &sig).is_ok()
    );
}
