use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// We test ApiClient directly by importing from the crate
// Since this is an integration test, we use the public API

#[tokio::test]
async fn get_with_auth_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/wallet"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"balance": 100.0, "totalEarned": 200.0, "totalSpent": 100.0})),
        )
        .mount(&server)
        .await;

    let client = ququer_client::api::ApiClient::new(&server.uri());
    let resp: serde_json::Value = client.get("/api/wallet", "test-token").await.unwrap();
    assert_eq!(resp["balance"], 100.0);
}

#[tokio::test]
async fn get_404_returns_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/wallet"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .mount(&server)
        .await;

    let client = ququer_client::api::ApiClient::new(&server.uri());
    let result: Result<serde_json::Value, _> = client.get("/api/wallet", "tok").await;
    let err = result.unwrap_err().to_string();
    assert!(err.contains("404"), "error should contain status code: {}", err);
}

#[tokio::test]
async fn get_500_returns_error_with_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/fail"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
        .mount(&server)
        .await;

    let client = ququer_client::api::ApiClient::new(&server.uri());
    let result: Result<serde_json::Value, _> = client.get("/api/fail", "tok").await;
    let err = result.unwrap_err().to_string();
    assert!(err.contains("500"));
    assert!(err.contains("internal error"));
}

#[tokio::test]
async fn post_with_auth_sends_body_and_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/game/g1/commit"))
        .and(header("Authorization", "Bearer my-token"))
        .and(header("content-type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .mount(&server)
        .await;

    let client = ququer_client::api::ApiClient::new(&server.uri());
    let body = json!({"hash": "abc", "signature": "def"});
    let resp: serde_json::Value = client
        .post("/api/game/g1/commit", &body, "my-token")
        .await
        .unwrap();
    assert_eq!(resp["ok"], true);
}

#[tokio::test]
async fn post_no_auth_omits_authorization_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/auth/register"))
        .respond_with(
            ResponseTemplate::new(201)
                .set_body_json(json!({"agentId": "a1", "name": "bot", "publicKey": "pk"})),
        )
        .mount(&server)
        .await;

    let client = ququer_client::api::ApiClient::new(&server.uri());
    let body = json!({"name": "bot", "publicKey": "pk"});
    let resp: serde_json::Value = client.post_no_auth("/api/auth/register", &body).await.unwrap();
    assert_eq!(resp["agentId"], "a1");
}

#[tokio::test]
async fn get_no_auth_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/games"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!([{"id": "rps", "name": "Rock Paper Scissors", "minPlayers": 2, "maxPlayers": 2}])),
        )
        .mount(&server)
        .await;

    let client = ququer_client::api::ApiClient::new(&server.uri());
    let resp: Vec<serde_json::Value> = client.get_no_auth("/api/games").await.unwrap();
    assert_eq!(resp.len(), 1);
    assert_eq!(resp[0]["id"], "rps");
}

#[tokio::test]
async fn delete_with_auth() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/api/matching/dequeue"))
        .and(header("Authorization", "Bearer tok"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "cancelled"})))
        .mount(&server)
        .await;

    let client = ququer_client::api::ApiClient::new(&server.uri());
    let resp: serde_json::Value = client.delete("/api/matching/dequeue", "tok").await.unwrap();
    assert_eq!(resp["status"], "cancelled");
}

#[tokio::test]
async fn url_construction_strips_trailing_slash() {
    let client = ququer_client::api::ApiClient::new("http://example.com/");
    assert_eq!(client.url("/api/test"), "http://example.com/api/test");
}
