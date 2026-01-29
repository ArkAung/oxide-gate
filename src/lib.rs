use axum::{
    extract::State,
    response::{sse::Event, IntoResponse, Sse},
    routing::{get, post},
    Json, Router,
};
use eventsource_stream::Eventsource;
use futures::StreamExt;
use serde_json::{json, Value};
use std::convert::Infallible;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Instant;

pub struct AppState {
    pub total_tokens_processed: AtomicUsize,
    pub request_count: AtomicUsize,
}

pub fn create_app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/messages", post(handle_claude_to_lmstudio))
        .route("/stats", get(handle_stats))
        .with_state(state)
}

async fn handle_claude_to_lmstudio(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let start_time = Instant::now();
    state.request_count.fetch_add(1, Ordering::Relaxed);

    let client = reqwest::Client::new();
    let messages = payload.get("messages").cloned().unwrap_or(json!([]));
    
    let lm_url = "http://localhost:1234/v1/chat/completions";

    let res = match client
        .post(lm_url)
        .json(&json!({
            "model": "local-model",
            "messages": messages,
            "stream": true
        }))
        .send()
        .await
    {
        Ok(response) => response,
        Err(_) => return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    };

    let mut first_token = true;
    let stream = res.bytes_stream().eventsource().map(move |event| {
        let res: Result<Event, Infallible> = match event {
            Ok(e) => {
                if e.data == "[DONE]" {
                    Ok(Event::default().data("{\"type\": \"message_stop\"}"))
                } else {
                    let data: Value = serde_json::from_str(&e.data).unwrap_or(json!({}));
                    let text = data["choices"][0]["delta"]["content"].as_str().unwrap_or("");

                    if first_token && !text.is_empty() {
                        println!("TTFT: {:?}", start_time.elapsed());
                        first_token = false;
                    }

                    state.total_tokens_processed.fetch_add(1, Ordering::Relaxed);

                    Ok(Event::default().data(json!({
                        "type": "content_block_delta",
                        "index": 0,
                        "delta": { "type": "text_delta", "text": text }
                    }).to_string()))
                }
            }
            Err(_) => Ok(Event::default().data("{\"type\": \"error\"}")),
        };
        res
    });

    Ok(Sse::new(stream))
}

async fn handle_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(json!({
        "total_tokens_processed": state.total_tokens_processed.load(Ordering::Relaxed),
        "total_requests_handled": state.request_count.load(Ordering::Relaxed)
    }))
}