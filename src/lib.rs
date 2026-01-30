use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use axum::{
    extract::State,
    response::{
        sse::{Event, Sse},
        IntoResponse,
        Response,
    },
    Json, Router,
    routing::{get, post},
    http::{StatusCode, HeaderMap, header},
};
use futures::stream::Stream;
use serde_json::{json, Value};
use tokio_stream::StreamExt as _;

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

pub async fn handle_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(json!({
        "requests": state.request_count.load(std::sync::atomic::Ordering::Relaxed),
        "tokens": state.total_tokens_processed.load(std::sync::atomic::Ordering::Relaxed),
    }))
}

pub async fn handle_claude_to_lmstudio(
    State(state): State<Arc<AppState>>,
    Json(mut body): Json<Value>,
) -> Response {
    state.request_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    // Strip Anthropic-specific fields that break OpenAI API
    if let Some(messages) = body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        for msg in messages {
            if let Some(content_array) = msg.get_mut("content").and_then(|c| c.as_array_mut()) {
                for block in content_array {
                    if let Some(obj) = block.as_object_mut() {
                        obj.remove("cache_control");
                    }
                }
            }
        }
    }

    // Extract max_tokens for usage tracking (optional)
    let max_tokens = body.get("max_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(1024);

    // Check if streaming is requested
    let stream_requested = body.get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !stream_requested {
        // Non-streaming request - not implemented yet
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": {
                    "type": "invalid_request_error",
                    "message": "Non-streaming requests not yet supported"
                }
            }))
        ).into_response();
    }

    // Forward to LM Studio with OpenAI format
    let client = reqwest::Client::new();
    let lm_studio_response = match client
        .post("http://localhost:1234/v1/chat/completions")
        .json(&json!({
            "model": body.get("model").unwrap_or(&json!("local-model")),
            "messages": body.get("messages"),
            "stream": true,
            "max_tokens": max_tokens,
        }))
        .send()
        .await
    {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("Failed to connect to LM Studio: {}", e);
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({
                    "error": {
                        "type": "api_error",
                        "message": format!("Failed to connect to LM Studio: {}", e)
                    }
                }))
            ).into_response();
        }
    };

    // Create the SSE stream
    let stream = convert_openai_to_anthropic_stream(lm_studio_response, state);
    
    // Return SSE response with proper headers
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "text/event-stream".parse().unwrap());
    headers.insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
    headers.insert(header::CONNECTION, "keep-alive".parse().unwrap());
    
    (headers, Sse::new(stream)).into_response()
}

fn convert_openai_to_anthropic_stream(
    response: reqwest::Response,
    state: Arc<AppState>,
) -> impl Stream<Item = Result<Event, std::convert::Infallible>> {
    let byte_stream = response.bytes_stream();
    
    async_stream::stream! {
        let msg_id = format!("msg_{}", uuid::Uuid::new_v4());
        let mut accumulated_text = String::new();
        let mut total_output_tokens = 0;

        // Event 1: message_start
        yield Ok(Event::default()
            .event("message_start")
            .data(json!({
                "type": "message_start",
                "message": {
                    "id": msg_id,
                    "type": "message",
                    "role": "assistant",
                    "model": "claude-3-5-sonnet-local",
                    "content": [],
                    "stop_reason": null,
                    "stop_sequence": null,
                    "usage": {
                        "input_tokens": 0,
                        "output_tokens": 0
                    }
                }
            }).to_string())
        );

        // Event 2: content_block_start
        yield Ok(Event::default()
            .event("content_block_start")
            .data(json!({
                "type": "content_block_start",
                "index": 0,
                "content_block": {
                    "type": "text",
                    "text": ""
                }
            }).to_string())
        );

        // Process OpenAI stream chunks
        let mut stream = byte_stream;
        let mut buffer = String::new();
        
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(bytes) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));
                    
                    // Process complete lines
                    while let Some(line_end) = buffer.find('\n') {
                        let line = buffer[..line_end].trim().to_string();
                        buffer = buffer[line_end + 1..].to_string();
                        
                        if line.is_empty() {
                            continue;
                        }
                        
                        // Parse SSE format: "data: {...}"
                        if let Some(json_str) = line.strip_prefix("data: ") {
                            if json_str.trim() == "[DONE]" {
                                break;
                            }
                            
                            // Parse OpenAI chunk
                            if let Ok(chunk) = serde_json::from_str::<Value>(json_str) {
                                // Extract content from delta
                                if let Some(content) = chunk
                                    .get("choices")
                                    .and_then(|c| c.get(0))
                                    .and_then(|c| c.get("delta"))
                                    .and_then(|d| d.get("content"))
                                    .and_then(|c| c.as_str())
                                {
                                    accumulated_text.push_str(content);
                                    total_output_tokens += 1; // Rough approximation
                                    
                                    // Event 3: content_block_delta
                                    yield Ok(Event::default()
                                        .event("content_block_delta")
                                        .data(json!({
                                            "type": "content_block_delta",
                                            "index": 0,
                                            "delta": {
                                                "type": "text_delta",
                                                "text": content
                                            }
                                        }).to_string())
                                    );
                                }
                                
                                // Check for finish_reason
                                if let Some(finish_reason) = chunk
                                    .get("choices")
                                    .and_then(|c| c.get(0))
                                    .and_then(|c| c.get("finish_reason"))
                                    .and_then(|f| f.as_str())
                                {
                                    if finish_reason != "null" && !finish_reason.is_empty() {
                                        tracing::info!("Stream finished with reason: {}", finish_reason);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Stream error: {}", e);
                    yield Ok(Event::default()
                        .event("error")
                        .data(json!({
                            "type": "error",
                            "error": {
                                "type": "api_error",
                                "message": format!("Stream error: {}", e)
                            }
                        }).to_string())
                    );
                    break;
                }
            }
        }

        // Update stats
        state.total_tokens_processed.fetch_add(
            total_output_tokens,
            std::sync::atomic::Ordering::SeqCst
        );

        // Event 4: content_block_stop
        yield Ok(Event::default()
            .event("content_block_stop")
            .data(json!({
                "type": "content_block_stop",
                "index": 0
            }).to_string())
        );

        // Event 5: message_delta (CRITICAL - required before message_stop)
        yield Ok(Event::default()
            .event("message_delta")
            .data(json!({
                "type": "message_delta",
                "delta": {
                    "stop_reason": "end_turn",
                    "stop_sequence": null
                },
                "usage": {
                    "output_tokens": total_output_tokens
                }
            }).to_string())
        );

        // Event 6: message_stop
        yield Ok(Event::default()
            .event("message_stop")
            .data(json!({
                "type": "message_stop"
            }).to_string())
        );
    }
}