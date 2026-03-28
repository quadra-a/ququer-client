use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use ququer_client::api::ApiClient;

fn mock_game_status_simultaneous() -> serde_json::Value {
    json!({
        "gameId": "game-1",
        "state": "active",
        "visibleState": {
            "currentPhase": {
                "id": "phase-1",
                "type": "simultaneous",
                "name": "action",
                "usesCommitReveal": true,
                "timeout": 30000
            }
        }
    })
}

fn mock_game_status_sequential() -> serde_json::Value {
    json!({
        "gameId": "game-1",
        "state": "active",
        "visibleState": {
            "currentPhase": {
                "id": "phase-2",
                "type": "sequential",
                "name": "bid",
                "usesCommitReveal": false,
                "timeout": 30000
            }
        }
    })
}

fn mock_game_status_no_phase() -> serde_json::Value {
    json!({
        "gameId": "game-1",
        "state": "finished",
        "visibleState": {}
    })
}

#[tokio::test]
async fn submit_cr_sends_commit_then_reveal() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/game/game-1/commit"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .expect(1)
        .named("commit")
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/game/game-1/reveal"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .expect(1)
        .named("reveal")
        .mount(&server)
        .await;

    let api = ApiClient::new(&server.uri());
    let key = ed25519_dalek::SigningKey::from_bytes(&[10u8; 32]);
    let data_str = r#"{"action":"rock"}"#;
    let data_value: serde_json::Value = serde_json::from_str(data_str).unwrap();

    let nonce = ququer_client::crypto::generate_nonce();
    let hash = ququer_client::crypto::commit_hash(data_str, &nonce);
    let signature = ququer_client::crypto::sign_bytes(&key, hash.as_bytes());

    // Verify commit hash matches protocol: SHA-256(JSON + ":" + nonce) → hex
    use sha2::{Digest, Sha256};
    let expected_input = format!("{}:{}", data_str, nonce);
    let expected_hash = hex::encode(Sha256::digest(expected_input.as_bytes()));
    assert_eq!(hash, expected_hash);

    // Verify signature is base64 and valid
    use base64::Engine;
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(&signature)
        .unwrap();
    assert_eq!(sig_bytes.len(), 64);
    let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes.try_into().unwrap());
    assert!(ed25519_dalek::Verifier::verify(&key.verifying_key(), hash.as_bytes(), &sig).is_ok());

    // Test commit API call
    let commit_body = json!({
        "gameId": "game-1",
        "phaseId": "phase-1",
        "hash": hash,
        "signature": signature,
        "timestamp": 12345
    });
    let resp: serde_json::Value = api
        .post("/api/game/game-1/commit", &commit_body, "test-token")
        .await
        .unwrap();
    assert_eq!(resp["ok"], true);

    // Test reveal API call (now includes timestamp)
    let reveal_sig = ququer_client::crypto::sign_bytes(
        &key,
        format!("{}:{}", data_str, nonce).as_bytes(),
    );
    let reveal_body = json!({
        "gameId": "game-1",
        "phaseId": "phase-1",
        "data": data_value,
        "nonce": nonce,
        "signature": reveal_sig,
        "timestamp": 12346
    });
    let resp: serde_json::Value = api
        .post("/api/game/game-1/reveal", &reveal_body, "test-token")
        .await
        .unwrap();
    assert_eq!(resp["ok"], true);
}

#[tokio::test]
async fn submit_action_sends_signed_action_with_action_type() {
    let server = MockServer::start().await;

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

    // Action payload now includes actionType and timestamp
    let action_body = json!({
        "gameId": "game-1",
        "phaseId": "phase-2",
        "actionType": "bid",
        "data": data_value,
        "signature": signature,
        "timestamp": 12345
    });
    let resp: serde_json::Value = api
        .post("/api/game/game-1/action", &action_body, "test-token")
        .await
        .unwrap();
    assert_eq!(resp["ok"], true);

    // Verify signature is base64
    use base64::Engine;
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(&signature)
        .unwrap();
    assert_eq!(sig_bytes.len(), 64);
}

#[tokio::test]
async fn game_status_response_parsed() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/game/game-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_game_status_no_phase()))
        .mount(&server)
        .await;

    let api = ApiClient::new(&server.uri());
    let status: ququer_client::types::GameStatusResponse =
        api.get("/api/game/game-1", "test-token").await.unwrap();
    assert_eq!(status.state, "finished");
}

#[tokio::test]
async fn game_status_with_phase_extracts_correctly() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/game/game-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_game_status_simultaneous()))
        .mount(&server)
        .await;

    let api = ApiClient::new(&server.uri());
    let status: ququer_client::types::GameStatusResponse =
        api.get("/api/game/game-1", "test-token").await.unwrap();
    assert_eq!(status.state, "active");

    // Extract phase from visibleState
    let phase: ququer_client::types::PhaseInfo =
        serde_json::from_value(status.visible_state["currentPhase"].clone()).unwrap();
    assert_eq!(phase.phase_type, "simultaneous");
    assert!(phase.uses_commit_reveal);
    assert_eq!(phase.id, "phase-1");
}

#[tokio::test]
async fn sse_game_events_parse_correctly() {
    let cases = vec![
        (r#"{"type":"all_committed","phase":"action"}"#, "all_committed"),
        (r#"{"type":"phase_result","phase":"action","result":{"winner":"a1"}}"#, "phase_result"),
        (r#"{"type":"game_end","winner":"a1","reason":"normal"}"#, "game_end"),
        (r#"{"type":"game_end","winner":null,"reason":"timeout"}"#, "game_end_draw"),
        (r#"{"type":"your_turn","phase":"bid"}"#, "your_turn"),
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
    let key = ed25519_dalek::SigningKey::from_bytes(&[20u8; 32]);
    let data = r#"{"action":"paper"}"#;
    let nonce = "fixed-nonce-for-test";

    let hash = ququer_client::crypto::commit_hash(data, nonce);
    let commit_sig = ququer_client::crypto::sign_bytes(&key, hash.as_bytes());

    // Hash is deterministic
    let reveal_hash = ququer_client::crypto::commit_hash(data, nonce);
    assert_eq!(hash, reveal_hash);

    // Signature is base64 and verifiable
    use base64::Engine;
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(&commit_sig)
        .unwrap();
    let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes.try_into().unwrap());
    assert!(ed25519_dalek::Verifier::verify(&key.verifying_key(), hash.as_bytes(), &sig).is_ok());
}

#[tokio::test]
async fn public_key_is_spki_base64_not_hex() {
    let key = ed25519_dalek::SigningKey::from_bytes(&[99u8; 32]);
    let pk = ququer_client::crypto::public_key_to_spki_base64(&key);

    // Should be base64, not hex
    use base64::Engine;
    let decoded = base64::engine::general_purpose::STANDARD.decode(&pk).unwrap();
    assert_eq!(decoded.len(), 44); // 12 byte SPKI prefix + 32 byte key

    // Should NOT be 64 chars hex
    assert_ne!(pk.len(), 64);
}
