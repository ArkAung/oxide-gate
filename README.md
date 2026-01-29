# Oxide-Gate

Oxide-Gate is a high-performance, local-first proxy designed to bridge **Claude Code** (Anthropic's CLI) with **LM Studio**. 

Built in Rust, it provides sub-millisecond overhead and real-time performance monitoring while ensuring your data never leaves your local machine. It translates Anthropic's Messages API into OpenAI-compatible requests on the fly.

## Features

- **Zero-Latency Proxying**: Built with Axum and Tokio for high-concurrency and non-blocking I/O.
- **Protocol Translation**: Seamlessly converts Anthropic SSE streams to OpenAI format.
- **Live Monitoring**: Real-time terminal logging of Time to First Token (TTFT) and throughput.
- **Stats Endpoint**: A built-in `/stats` JSON endpoint for performance auditing.
- **Privacy Focused**: No data collection, no external telemetry.

## Prerequisites

- [Rust](https://rustup.rs/) (Stable)
- [LM Studio](https://lmstudio.ai/) running a local server (default: port 1234)
- Claude code

## Quick Start

### 1. Build and Run Oxide-Gate
```bash
git clone [https://github.com/arkaung/oxide-gate](https://github.com/your-username/oxide-gate)
cd oxide-gate
cargo run --release
```
The bridge will start on http://127.0.0.1:5005.

### 2. Configure Claude Code

Redirect the Claude CLI traffic to your local bridge:
```bash
export ANTHROPIC_BASE_URL="[http://127.0.0.1:5005/v1](http://127.0.0.1:5005/v1)"
export ANTHROPIC_API_KEY="local"
claude
```

## Development and Testing
### Run Integration Tests
```bash
cargo test
```

## Check Performance Stats

While the proxy is running, you can query live metrics:
```bash
curl [http://127.0.0.1:5005/stats](http://127.0.0.1:5005/stats)
```