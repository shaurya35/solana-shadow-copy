# Implementation guide

## Goal

- Build a read-only Solana transaction diagnostics service.
- Accept one finalized transaction signature.
- Return an evidence-backed diagnosis through HTTP and Telegram.
- Support two confirmed classifications and an honest unknown fallback.

## Version `0.1` scope

### Required

- Rust diagnostics library.
- Solana `getTransaction` RPC client.
- Transaction and instruction normalization.
- Nested program-log attribution.
- Insufficient-lamports classifier.
- Jupiter `6001` classifier bound to the verified program ID.
- Unknown program and transaction fallbacks.
- HTTP diagnosis endpoint.
- Telegram renderer and demo bot.
- Three sanitized mainnet fixtures.
- Focused regression and API tests.
- Docker image and concise README.

### Deferred

- Trade execution, signing, simulation, or retries.
- Wallet connection or private-key handling.
- Additional program classifiers.
- Database, persistent cache, and analytics dashboard.
- Web interface.
- Multi-provider failover and production-scale infrastructure.

## Repository layout

```text
Cargo.toml
Cargo.lock
.env.example
src/
  lib.rs
  config.rs
  domain.rs
  rpc.rs
  normalize.rs
  invocation_logs.rs
  classify.rs
  render_telegram.rs
  api.rs
  bin/
    server.rs
    telegram_bot.rs
fixtures/
  manifest.json
  insufficient_transfer.json
  jupiter_slippage_6001.json
  unknown_custom_error.json
tests/
  fixture_regression.rs
  api.rs
  telegram_rendering.rs
Dockerfile
```

## Module responsibilities

| Module | Responsibility |
|---|---|
| `config` | Load and validate RPC, HTTP, Telegram, CORS, and logging configuration |
| `domain` | Own stable diagnosis, evidence, confidence, instruction, and error types |
| `rpc` | Validate signatures and call finalized `getTransaction` with timeouts |
| `normalize` | Convert RPC JSON into stable account, instruction, fee, error, and log data |
| `invocation_logs` | Parse nested `invoke`, `success`, and `failed` program lines |
| `classify` | Apply deterministic classifiers in fixed priority order |
| `render_telegram` | Escape and render a diagnosis as a Telegram message fragment |
| `api` | Expose diagnosis, example-list, and health routes |
| `server` | Start the HTTP service and shared dependencies |
| `telegram_bot` | Parse commands, call the API, and send responses with its own token |

## External contracts

### Solana RPC

- Method: `getTransaction`.
- Commitment: `finalized`.
- Encoding: `jsonParsed`.
- Maximum supported transaction version: `0`.
- Map `result: null` to transaction unavailable.
- Map missing metadata to metadata unavailable.
- Bound response and log sizes.
- Never log an authenticated RPC URL.

### HTTP API

- `POST /v1/diagnoses` analyzes one signature.
- `GET /v1/examples` lists the public example signatures.
- `GET /health` reports process health without an RPC call.
- Return exact lamport values as decimal strings.
- Return known and unknown diagnoses with the same response shape.
- Return a Telegram message fragment without `chat_id` or bot token.

### Telegram bot

- `/start` explains the read-only purpose.
- `/examples` shows the three public example signatures.
- `/explain <signature>` requests a live diagnosis.
- `/help` shows supported input and limitations.
- Use long polling for the initial deployment.

## Classification order

1. Explicit `Transfer: insufficient lamports <available>, need <required>` runtime evidence.
2. Verified Jupiter program ID plus custom error `6001`.
3. Attributed custom program error with no verified mapping.
4. Generic transaction error.

## Evidence rules

- A confirmed diagnosis requires explicit runtime evidence or an exact program-ID and documented-code mapping.
- A numeric custom error never inherits the meaning of the same number from another program.
- Unknown is a valid analysis result.
- Preserve the failed instruction, program, code, fee, and bounded logs when available.
- Never generate a diagnosis or remediation with an LLM.
- Never promise that a retry will succeed.

## Build sequence

### 1. Scaffold

- Verify current crate versions from official documentation.
- Create one Rust library and `server` and `telegram_bot` binaries.
- Add typed configuration and `.env.example`.
- Gate: `cargo check` passes and secrets remain untracked.

### 2. Add fixtures

- Fetch the three public transactions through `getTransaction`.
- Compare fees, errors, programs, and logs with Solana Explorer.
- Remove provider metadata and credentials.
- Record expected results and fixture hashes.
- Gate: each fixture is manually verified.

### 3. Implement the diagnostics core

- Normalize legacy and version `0` transactions.
- Resolve the failed top-level instruction safely.
- Parse nested program invocation logs with a checked stack.
- Attribute the deepest failing program only when logs support it.
- Implement exact integer lamport-to-SOL formatting.
- Implement the four classification outcomes.
- Gate: all fixture regressions pass offline.

### 4. Implement the live API

- Add the RPC client with connect and total timeouts.
- Add signature validation before outbound work.
- Add diagnosis, example-list, and health routes.
- Add typed validation, unavailable, provider, and internal errors.
- Gate: all three public signatures work through `curl`.

### 5. Implement Telegram

- Render HTML-safe messages with an Explorer button.
- Add API client and command handlers.
- Keep token and `chat_id` inside the bot process.
- Add clear invalid-input and provider-error messages.
- Gate: commands work from a real Telegram account.

### 6. Package and release

- Run format, lint, tests, and release build.
- Build a non-root Docker image.
- Deploy API and long-polling bot.
- Verify the hosted bot from a separate account.
- Complete README examples and limitations.

## Required tests

- Signature validation rejects wallet keys, URLs, malformed base58, and oversized input.
- Legacy and version `0` fixtures normalize without panics.
- Failed instruction indexes are bounds checked.
- Invocation parsing handles nested, missing, and malformed log sequences.
- Insufficient-lamports amounts are exact.
- Jupiter `6001` matches only the verified program.
- The unknown fixture remains unknown.
- Lamport formatting uses integer arithmetic.
- Telegram renderer escapes program-controlled text.
- Invalid API input does not call RPC.
- Null and timed-out RPC responses map to stable HTTP errors.

## Verification commands

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo build --release --bins
docker build -t solana-trade-diagnostics:local .
```

## Release checklist

- [x] Three real fixtures are checked and reproducible.
- [x] Two confirmed classifications and both unknown fallbacks work.
- [x] Wrong-program Jupiter code remains unknown.
- [x] Live HTTP requests work for all example signatures.
- [x] Telegram commands, diagnoses, invalid-input handling, and Explorer buttons are verified with an operator-provided bot token.
- [x] RPC URL and Telegram token are absent from Git and logs.
- [x] Local non-root Docker build and container smoke test pass.
- [x] Hosted CI passes on the published commit.
- [x] README describes supported behavior and limitations accurately.

## Primary references

- Solana `getTransaction`: <https://solana.com/docs/rpc/http/gettransaction>
- Solana RPC JSON structures: <https://solana.com/docs/rpc/json-structures>
- Jupiter common errors: <https://dev.jup.ag/docs/swap/common-errors>
- Telegram `sendMessage`: <https://core.telegram.org/bots/api#sendmessage>
- Telegram inline keyboards: <https://core.telegram.org/bots/api#inlinekeyboardmarkup>
