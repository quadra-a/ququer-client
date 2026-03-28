use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use ququer_client::api::ApiClient;

#[tokio::test]
async fn register_flow_creates_agent() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/auth/register"))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(json!({
                "agentId": "agent-new",
                "name": "test-bot",
                "publicKey": "aabbccdd"
            })),
        )
        .expect(1)
        .named("register")
        .mount(&server)
        .await;

    let api = ApiClient::new(&server.uri());
    let body = json!({"name": "test-bot", "publicKey": "aabbccdd"});
    let resp: serde_json::Value = api.post_no_auth("/api/auth/register", &body).await.unwrap();
    assert_eq!(resp["agentId"], "agent-new");
    assert_eq!(resp["name"], "test-bot");
}

#[tokio::test]
async fn register_duplicate_key_returns_409() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/auth/register"))
        .respond_with(
            ResponseTemplate::new(409).set_body_string("public key already registered"),
        )
        .mount(&server)
        .await;

    let api = ApiClient::new(&server.uri());
    let body = json!({"name": "bot", "publicKey": "existing-key"});
    let result: Result<serde_json::Value, _> = api.post_no_auth("/api/auth/register", &body).await;
    let err = result.unwrap_err().to_string();
    assert!(err.contains("409"));
}

#[tokio::test]
async fn full_register_then_login_flow() {
    let server = MockServer::start().await;
    let key = ed25519_dalek::SigningKey::from_bytes(&[8u8; 32]);
    let pub_key = hex::encode(key.verifying_key().as_bytes());

    // Register
    Mock::given(method("POST"))
        .and(path("/api/auth/register"))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(json!({
                "agentId": "agent-8",
                "name": "bot-8",
                "publicKey": pub_key
            })),
        )
        .mount(&server)
        .await;

    // Challenge
    Mock::given(method("GET"))
        .and(path("/api/auth/challenge"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"challenge": "ch-xyz"})),
        )
        .mount(&server)
        .await;

    // Login
    Mock::given(method("POST"))
        .and(path("/api/auth/login"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"token": "tok-123", "expiresAt": "9999999999"})),
        )
        .mount(&server)
        .await;

    let api = ApiClient::new(&server.uri());

    // Step 1: Register
    let reg_body = json!({"name": "bot-8", "publicKey": pub_key});
    let reg_resp: serde_json::Value = api.post_no_auth("/api/auth/register", &reg_body).await.unwrap();
    assert_eq!(reg_resp["agentId"], "agent-8");

    // Step 2: Login
    let cache = ququer_client::auth::login(&api, &key, "agent-8").await.unwrap();
    assert_eq!(cache.token, "tok-123");
    assert_eq!(cache.agent_id, "agent-8");
}
