# Solana Trade Diagnostics

Read-only diagnostics for finalized Solana transactions. Submit a transaction signature and receive a structured, evidence-backed explanation plus a Telegram-ready message.

The service currently recognizes:

- Explicit insufficient-lamports transfer failures.
- Jupiter Swap program error `6001` (`SlippageToleranceExceeded`), only when the failing program ID matches Jupiter.
- Unmapped custom program errors without guessing their meaning.
- Generic transaction failures and successful transactions.

It never connects a wallet, signs, simulates, retries, or submits a transaction.

## Architecture

```text
signature -> HTTP API -> finalized getTransaction -> normalization
          -> invocation attribution -> deterministic classifiers
          -> structured JSON + Telegram-safe message
```

The demo Telegram bot is a separate process. It calls the public HTTP contract instead of importing classification logic or receiving the RPC credential.

## Run locally

Requirements: Rust `1.96+` and a Solana mainnet RPC URL.

```bash
cp .env.example .env
```

Set `SOLANA_RPC_URL` in `.env`, then start the API:

```bash
cargo run --bin server
```

Check it:

```bash
curl http://127.0.0.1:8080/health/live
```

Diagnose a verified example:

```bash
curl -sS http://127.0.0.1:8080/v1/diagnoses \
  -H 'Content-Type: application/json' \
  --data '{"signature":"5DLZ8sA6FPFThpKiD2QGzX3ufPjbV3hRk7Xf7P1A7m3kgB6KX6wcRcND4BdWXNwfbJxV1XNo7JooZJsj7GBrcuYn"}' \
  | jq
```

The response includes the category, confidence, failed instruction, fee, bounded log tail, evidence, documentation sources, Explorer link, and Telegram payload.

## Verified examples

| Case | Expected category | Signature |
|---|---|---|
| Insufficient SOL | `insufficient_transfer_balance` | `5DLZ8sA6FPFThpKiD2QGzX3ufPjbV3hRk7Xf7P1A7m3kgB6KX6wcRcND4BdWXNwfbJxV1XNo7JooZJsj7GBrcuYn` |
| Jupiter slippage | `jupiter_slippage_tolerance_exceeded` | `5UntMZRg4ChcYbsY5orMi3ez7uvc64GdheL8R39sWiPRfCeSqQzhK9JYGQ1eeri9LhFYKDVL84KKPQamQJy7V1xR` |
| Unmapped custom error | `unknown_program_error` | `42CkCpX9maDhJmFNZj5dDCS4uoo1ELSqyosuQp9BAU4e8xMbRx3K39DnX3UctgbeRdjukAJo9UTpwGHgKq8nZhUM` |

`fixtures/` contains the sanitized RPC responses, expected facts, Explorer verification links, and SHA-256 hashes used by offline regression tests.

## End-to-end Telegram proof

Tested on 24 September 2026 through a real Telegram bot, the local diagnostics API, Helius RPC, and finalized Solana mainnet transactions.

<p>
  <img src="docs/assets/telegram-insufficient-sol.png" width="49%" alt="Confirmed insufficient SOL diagnosis in Telegram">
  <img src="docs/assets/telegram-jupiter-slippage.png" width="49%" alt="Confirmed Jupiter slippage diagnosis in Telegram">
</p>

The complete test record includes the start flow, all three classifications, invalid-input handling, and Explorer verification: [end-to-end test evidence](docs/END_TO_END_TEST.md).

## Telegram demo bot

Create a bot with BotFather and set `TELEGRAM_BOT_TOKEN` in `.env`. Keep the API running, then start:

```bash
cargo run --bin telegram_bot
```

Commands:

- `/start`
- `/help`
- `/examples`
- `/explain <transaction-signature>`

The token and `chat_id` never enter the diagnostics API.

## Docker

Start the API:

```bash
docker compose up --build api
```

Start the optional Telegram bot after setting its token:

```bash
docker compose --profile telegram up --build
```

## Verification

```bash
./scripts/verify-fixtures.sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo build --release --bins
docker build -t solana-trade-diagnostics:local .
```

## Limits

- Mainnet-beta only.
- Legacy and version `0` transactions only.
- Finalized, landed transactions only; bot outages and quote failures without a transaction signature are outside scope.
- Three initial classifier outcomes; unknown is intentional when no program-specific mapping is verified.
- One RPC provider per process and no persistent cache in version `0.1`.

Detailed design and integration contracts are in [docs/README.md](docs/README.md).
