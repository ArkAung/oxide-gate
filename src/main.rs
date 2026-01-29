use std::sync::{atomic::AtomicUsize, Arc};
use oxide_gate::{create_app, AppState}; 

#[tokio::main]
async fn main() {
    let shared_state = Arc::new(AppState {
        total_tokens_processed: AtomicUsize::new(0),
        request_count: AtomicUsize::new(0),
    });

    let app = create_app(shared_state);

    let port = 5005;
    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to port 5005. Is it already in use?");
    
    println!("Oxide-Gate Proxy active on http://{}", addr);
    println!("Redirecting Claude CLI traffic to local LM Studio server...");

    axum::serve(listener, app).await.unwrap();
}