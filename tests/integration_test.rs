use axum_test::TestServer;
use oxide_gate::{create_app, AppState};
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

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
        "total_tokens_processed": 100,
        "total_requests_handled": 5
    }));
}