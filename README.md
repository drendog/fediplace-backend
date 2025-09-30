# FediPlace Backend

Collaborative pixel painting platform built with Rust, inspired by r/place and wplace. Designed for federated worlds - each instance hosts its own canvas and federates with other instances (federation not yet implemented).

## Quick Start

```bash
# Start services
docker compose up -d

# Run migrations
sqlx migrate run

# Start server
cargo run
```

Server runs on <http://localhost:8000> by default. API docs at `/docs/` with `cargo run --features docs`.
Make sure to set up `.env` from `.env.example` with your own secrets, and make sure to set up `config.toml` from `config.toml.example`.

## Architecture

Strict hexagonal architecture (ports and adapters) with dependency inversion: Domain (pure logic) ← Application (use cases) ← Adapters ← Server (composition).
Each layer is separated into its own crate, and architectural violations are enforced via `deny.toml` and `clippy.toml`.
