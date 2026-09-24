# Implementation notes

## What is built

- A Rust library for normalization, failure attribution, classification, and Telegram rendering.
- An Axum HTTP server backed by Solana `getTransaction`.
- A separate Telegram long-polling bot that calls the HTTP API.
- Three sanitized mainnet fixtures with recorded hashes and expected results.
- Offline regression tests, API tests, Docker packaging, and GitHub Actions CI.

## Repository layout

```text
src/
  api.rs                 HTTP routes and error mapping
  classify.rs            deterministic diagnosis rules
  config.rs              environment parsing and secret redaction
  domain.rs              public diagnosis types
  examples.rs            verified demo signatures
  invocation_logs.rs     nested program-call parser
  normalize.rs           RPC response normalization
  render_telegram.rs     safe Telegram HTML
  rpc.rs                  signature validation and RPC client
  telegram.rs             Telegram polling and commands
  bin/server.rs           API entry point
  bin/telegram_bot.rs     bot entry point
fixtures/                 sanitized RPC responses
tests/                    fixture regression tests
docs/assets/              end-to-end screenshots
```

## Classification rules

Rules run in this order:

1. Exact insufficient-lamports runtime log.
2. Jupiter Swap program plus documented custom error `6001`.
3. Unmapped custom program error.
4. Generic transaction error.

The output is deterministic. No model or external explanation service is used.

## Configuration

| Variable | Purpose | Default |
|---|---|---|
| `SOLANA_RPC_URL` | Solana mainnet RPC endpoint | Required |
| `RPC_CONNECT_TIMEOUT_MS` | RPC connect timeout | `2000` |
| `RPC_REQUEST_TIMEOUT_MS` | Whole RPC request timeout | `8000` |
| `API_BIND_ADDR` | HTTP listen address | `0.0.0.0:8080` |
| `DIAGNOSTICS_API_URL` | API base URL used by the bot | Required for bot |
| `TELEGRAM_BOT_TOKEN` | Token from BotFather | Required for bot |
| `RUST_LOG` | Log filter | `info` in the example file |

`.env` is ignored by Git. Configuration errors never include secret values.

## Run locally

```bash
cp .env.example .env
cargo run --bin server
```

In a second terminal:

```bash
cargo run --bin telegram_bot
```

The bot exits during startup if its token is malformed or Telegram rejects `getMe`.

## Verification

```bash
./scripts/verify-fixtures.sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --locked
cargo build --release --bins --locked
docker build -t solana-trade-diagnostics:local .
```

## Current limits

- Mainnet-beta only.
- Legacy and version `0` transactions only.
- One RPC provider.
- No cache, database, web interface, authentication, or per-user rate limit.
- Only landed, finalized transactions can be diagnosed.
- Three showcase failure classes plus a generic fallback.

## References

- Solana `getTransaction`: <https://solana.com/docs/rpc/http/gettransaction>
- Solana RPC JSON structures: <https://solana.com/docs/rpc/json-structures>
- Jupiter common errors: <https://dev.jup.ag/docs/swap/common-errors>
- Telegram Bot API: <https://core.telegram.org/bots/api>
