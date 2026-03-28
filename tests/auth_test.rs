use ed25519_dalek::SigningKey;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use ququer_client::api::ApiClient;
use ququer_client::auth;

#[tokio::test]
async fn login_flow_challenge_then_token() {
    let server = MockServer::start().await;
    let key = SigningKey::from_bytes(&[5u8; 32]);

    // Mock challenge
    Mock::given(method("GET"))
        .and(path("/api/auth/challenge"))
        .and(query_param("agentId", "agent-1"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"challenge": "random-challenge-123", "expiresAt": 9999999999u64})),
        )
        .mount(&server)
        .await;

    // Mock login
    Mock::given(method("POST"))
        .and(path("/api/auth/login"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"token": "jwt-token-abc", "expiresAt": 9999999999u64})),
        )
        .expect(1)
        .mount(&server)
        .await;

    let api = ApiClient::new(&server.uri());
    let cache = auth::login(&api, &key, "agent-1").await.unwrap();

    assert_eq!(cache.token, "jwt-token-abc");
    assert_eq!(cache.agent_id, "agent-1");
    assert_eq!(cache.expires_at, 9999999999u64);
}

#[tokio::test]
async fn login_challenge_failure_propagates() {
    let server = MockServer::start().await;
    let key = SigningKey::from_bytes(&[5u8; 32]);

    Mock::given(method("GET"))
        .and(path("/api/auth/challenge"))
        .respond_with(ResponseTemplate::new(404).set_body_string("agent not found"))
        .mount(&server)
        .await;

    let api = ApiClient::new(&server.uri());
    let result = auth::login(&api, &key, "nonexistent").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("404"));
}

#[tokio::test]
async fn login_bad_credentials_propagates() {
    let server = MockServer::start().await;
    let key = SigningKey::from_bytes(&[5u8; 32]);

    Mock::given(method("GET"))
        .and(path("/api/auth/challenge"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"challenge": "ch", "expiresAt": 9999999999u64})),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/auth/login"))
        .respond_with(ResponseTemplate::new(401).set_body_string("invalid signature"))
        .mount(&server)
        .await;

    let api = ApiClient::new(&server.uri());
    let result = auth::login(&api, &key, "agent-1").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("401"));
}
