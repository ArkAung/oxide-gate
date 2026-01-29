use ax_utils::response::IntoResponse;
use axum::{routing::post, Json, Router, response::Sse, response::sse::Event};
use futures::StreamExt;
use serde_json::{json, Value};
use std::time::{Instant, Duration};
use tokio_stream::Stream;
use eventsource_stream::Eventsource;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/v1/messages", post(handle_proxy));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:5005").await.unwrap();
    println!("🚀 High-Performance Bridge live at http://127.0.0.1:5005");
    axum::serve(listener, app).await.unwrap();
}

async fn handle_proxy(Json(payload): Json<Value>) -> impl IntoResponse {
    let start_time = Instant::now();
    let client = reqwest::Client::new();

    // 1. Translation: Claude format -> OpenAI (LM Studio) format
    let messages = payload["messages"].clone();
    let openai_body = json!({
        "model": "local-model", // LM Studio uses whatever is loaded
        "messages": messages,
        "stream": true
    });

    // 2. Forward to LM Studio
    let res = client.post("http://localhost:1234/v1/chat/completions")
        .json(&openai_body)
        .send()
        .await
        .unwrap();

    let mut ttft_recorded = false;
    let mut token_count = 0;
    
    // 3. SSE Stream Transformation
    let stream = res.bytes_stream().eventsource().map(move |event| {
        match event {
            Ok(e) => {
                if !ttft_recorded {
                    println!("⏱️ Time to First Token: {:?}", start_time.elapsed());
                    ttft_recorded = true;
                }
                
                if e.data == "[DONE]" {
                    return Ok(Event::default().data("event: message_stop\ndata: {\"type\": \"message_stop\"}"));
                }

                let data: Value = serde_json::from_str(&e.data).unwrap_or(json!({}));
                let text = data["choices"][0]["delta"]["content"].as_str().unwrap_or("");
                
                token_count += 1;
                // Simple Live Stat: Log every 10 tokens
                if token_count % 10 == 0 {
                    let elapsed = start_time.elapsed().as_secs_f64();
                    println!("📊 Speed: {:.2} tokens/sec", token_count as f64 / elapsed);
                }

                // Format back to Anthropic SSE style
                let anthropic_event = json!({
                    "type": "content_block_delta",
                    "index": 0,
                    "delta": { "type": "text_delta", "text": text }
                });

                Ok(Event::default().data(anthropic_event.to_string()))
            }
            Err(_) => Ok(Event::default().data("error")),
        }
    });

    Sse::new(stream)
}