use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use oxide_gate::{create_app, AppState};
use tower_http::trace::{TraceLayer, DefaultMakeSpan, DefaultOnResponse};
use tower_http::LatencyUnit;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() {
    // Initialize enhanced logging
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "oxide_gate=info,tower_http=debug,axum=debug".into())
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Oxide-Gate Proxy starting up...");

    // Create shared state
    let shared_state = Arc::new(AppState {
        total_tokens_processed: AtomicUsize::new(0),
        request_count: AtomicUsize::new(0),
    });

    // Build app with enhanced tracing
    let app = create_app(shared_state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(tracing::Level::INFO))
                .on_response(
                    DefaultOnResponse::new()
                        .level(tracing::Level::INFO)
                        .latency_unit(LatencyUnit::Millis)
                )
        );

    // Configuration
    let addr = "127.0.0.1:5005";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    
    tracing::info!("Oxide-Gate Proxy is running!");
    tracing::info!("Listening on http://{}", addr);
    tracing::info!("Forwarding to LM Studio at http://localhost:1234");
    tracing::info!("Stats endpoint: http://{}/stats", addr);
    tracing::info!("");
    tracing::info!("Configure Claude CLI:");
    tracing::info!("  export ANTHROPIC_BASE_URL=http://127.0.0.1:5005");
    tracing::info!("  export ANTHROPIC_API_KEY=dummy-key");
    tracing::info!("");

    axum::serve(listener, app).await.unwrap();
}