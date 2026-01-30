# Oxide-Gate

Oxide-Gate is a high-performance, local-first proxy designed to bridge **Claude Code** (Anthropic's CLI) with **LM Studio**. 

Built in Rust, it provides sub-millisecond overhead and real-time performance monitoring while ensuring your data never leaves your local machine. It translates Anthropic's Messages API into OpenAI-compatible requests on the fly.

## Quick Start

### 1. Build and Run Oxide-Gate
```bash
git clone https://github.com/arkaung/oxide-gate
cd oxide-gate
cargo run --release
```
The bridge will start on http://127.0.0.1:5005.

### 2. Configure Claude Code

Redirect the Claude CLI traffic to your local bridge:
```bash
export ANTHROPIC_BASE_URL="http://127.0.0.1:5005"
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
curl http://127.0.0.1:5005/stats
```