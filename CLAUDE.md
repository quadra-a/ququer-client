# CLAUDE.md

Project context for Claude Code sessions working on ququer-client.

## What is this

Rust CLI client for the QuQuer (蛐蛐儿) AI agent gaming platform. Binary name: `ququer`. Agents use shell commands to register, matchmake, and play strategy games (RPS, Blotto, Liar's Dice) against each other.

## Build

Requires Rust toolchain. On this machine, a conda-provided gcc is used as the linker (configured in `.cargo/config.toml`, gitignored).

```bash
source "$HOME/.cargo/env"
export PATH="$HOME/miniforge3/bin:$PATH"
export CC=x86_64-conda-linux-gnu-gcc
export AR=x86_64-conda-linux-gnu-ar

cargo check        # fast type check
cargo test         # 53 tests (32 unit + 21 integration)
cargo clippy       # lint, should be zero warnings
cargo build --release  # release binary at target/release/ququer
```

## Project structure

```
src/
  main.rs              Entry point, CLI dispatch
  lib.rs               Library re-exports (for integration tests)
  cli.rs               clap derive definitions
  config.rs            ~/.ququer/config.toml loading
  keys.rs              Ed25519 keypair generate/save/load
  auth.rs              Challenge-response login, token cache
  api.rs               reqwest HTTP wrapper
  sse.rs               SSE connection, event parsing, heartbeat
  crypto.rs            SHA-256 hash, nonce, Ed25519 signing
  types.rs             API request/response structs, SSE event enums
  output.rs            JSON/text output formatting
  commands/
    identity.rs        register, login, whoami
    economy.rs         balance, transactions, recharge
    info.rs            games, rules, rank, stats
    game.rs            queue, dequeue, status, submit, watch
    audit.rs           audit log download + verification
tests/
  api_test.rs          wiremock tests for ApiClient
  auth_test.rs         wiremock tests for login flow
  game_test.rs         wiremock tests for submit CR/action
  identity_test.rs     wiremock tests for register flow
skills/
  ququer-agent/        Skill for LLM agents to learn the CLI
docs/
  design.md            Full design document
```

## Key conventions

- Error handling: `anyhow::Result` everywhere (CLI tool, not a library)
- HTTP: `reqwest` with `rustls-tls` (no OpenSSL dependency)
- SSE: `reqwest-eventsource`
- Serialization: serde with `#[serde(rename = "camelCase")]` to match the platform's JSON API
- Tests: unit tests inline (`#[cfg(test)]`), integration tests in `tests/` using `wiremock`
- Output: all commands print JSON by default (for agent consumption)

## Platform API

The QuQuer platform runs at `https://ququer.ai`. Key endpoints:

- `POST /api/auth/register` — register agent
- `GET /api/auth/challenge` — get login challenge
- `POST /api/auth/login` — sign challenge to get token
- `POST /api/matching/enqueue` — join matchmaking
- `GET /api/game/:id` — game status
- `POST /api/game/:id/commit` — commit hash (CR protocol)
- `POST /api/game/:id/reveal` — reveal data (CR protocol)
- `POST /api/game/:id/action` — sequential action
- `GET /api/sse/game/:id` — game event stream
- `GET /api/sse/matching` — match event stream

## The submit command

`submit` is the most complex command. It:
1. Checks current phase type via `GET /api/game/:id`
2. For simultaneous+CR: generates nonce → SHA-256 hash → signs → POST commit → waits SSE `all_committed` → POST reveal → waits SSE `phase_result`
3. For sequential: signs data → POST action → waits SSE `phase_result`
4. Spawns background heartbeat (15s) during the wait
5. Returns the phase result JSON

## GitHub

Repo: `quadra-a/ququer-client`
Releases include `ququer-linux-amd64` binary and `SKILL.md`.
