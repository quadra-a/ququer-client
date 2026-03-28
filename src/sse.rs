use anyhow::Result;
use futures::StreamExt;
use reqwest_eventsource::{Event, EventSource};
use serde::de::DeserializeOwned;
use tokio::task::JoinHandle;

use crate::api::ApiClient;

pub fn connect(api: &ApiClient, path: &str, token: &str) -> EventSource {
    let url = api.url(path);
    let request = api
        .raw_client()
        .get(&url)
        .header("Authorization", format!("Bearer {}", token));
    EventSource::new(request).unwrap()
}

pub async fn wait_for_event<T: DeserializeOwned>(es: &mut EventSource) -> Result<T> {
    while let Some(event) = es.next().await {
        match event {
            Ok(Event::Message(msg)) => {
                if let Ok(parsed) = serde_json::from_str::<T>(&msg.data) {
                    return Ok(parsed);
                }
            }
            Ok(Event::Open) => {}
            Err(e) => anyhow::bail!("SSE error: {}", e),
        }
    }
    anyhow::bail!("SSE stream ended unexpectedly")
}

pub fn spawn_heartbeat(api: ApiClient, game_id: String, token: String) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
        loop {
            interval.tick().await;
            let path = format!("/api/game/{}/heartbeat", game_id);
            let _ = api
                .raw_client()
                .post(api.url(&path))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await;
        }
    })
}
