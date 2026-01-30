use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use serde_json::json;
use axum_test::TestServer;
use oxide_gate::{create_app, AppState};

#[tokio::test]
async fn test_stats_endpoint() {
    let state = Arc::new(AppState {
        total_tokens_processed: AtomicUsize::new(100),
        request_count: AtomicUsize::new(5),
    });
    
    let app = create_app(state);
    let server = TestServer::new(app).expect("Failed to create test server");
    
    let response = server.get("/stats").await;
    response.assert_status_ok();
    
    response.assert_json(&json!({
        "requests": 5,
        "tokens": 100
    }));
}

#[tokio::test]
async fn test_health_check() {
    let state = Arc::new(AppState {
        total_tokens_processed: AtomicUsize::new(0),
        request_count: AtomicUsize::new(0),
    });
    
    let app = create_app(state);
    let server = TestServer::new(app).expect("Failed to create test server");
    
    let response = server.get("/stats").await;
    response.assert_status_ok();
}