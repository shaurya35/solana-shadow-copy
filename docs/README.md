# Solana Trade Diagnostics — documentation index

## Project status

- Status: implemented and end-to-end tested through a real Telegram bot against all three mainnet examples, plus invalid-input handling.
- Working name: `solana-trade-diagnostics`.
- Repository: `solana-trade-diagnostics`.
- Target chain: Solana mainnet-beta only for version `0.1`.
- Primary implementation: Rust.
- Required product surface: HTTP API, Telegram connector, and demo Telegram bot; web demo is optional polish.
- Safety boundary: read-only analysis; no wallet connection, signing, simulation, or transaction submission.

## One-line product

- Turn a failed Solana transaction signature into an evidence-backed explanation and a Telegram-ready response.

## Documentation map

| Document | Purpose |
|---|---|
| [PRODUCT_SPEC.md](./PRODUCT_SPEC.md) | User problem, product behavior, scope, non-goals, claims, and success criteria |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | System boundaries, data flow, modules, domain model, classifiers, confidence, and operational behavior |
| [API_AND_TELEGRAM.md](./API_AND_TELEGRAM.md) | HTTP contract, Telegram rendering contract, demo-bot flow, and possible existing-bot integration |
| [IMPLEMENTATION.md](./IMPLEMENTATION.md) | Generic build order, module responsibilities, verification gates, and release checklist |

## Fixed version `0.1` decisions

| Area | Decision |
|---|---|
| Input | One base58 Solana transaction signature |
| Analysis source | Solana JSON-RPC `getTransaction` with parsed transaction data and metadata |
| Commitment | Request `finalized`; report unavailable if the RPC returns `null` |
| Output | Structured diagnosis, evidence, confidence, fee, Explorer URL, and Telegram message model |
| Showcase classifiers | Insufficient transfer balance, Jupiter `6001`, unknown program error, and unknown transaction error |
| Unknown errors | Preserve program, code, instruction, and logs; never guess the meaning |
| Telegram integration | Return a Telegram-compatible message body; the caller retains its own bot token and `chat_id` |
| Demo bot | `/explain <signature>` plus preloaded `/examples` |
| Demo page | Optional after the required API and Telegram showcase works |
| Persistence | No database or runtime cache in `0.1`; checked-in sanitized fixtures are used for tests |
| Execution | No transaction building, simulation, signing, sending, retrying, or automated trading |
| Product claim | Read-only Solana diagnostics prototype for Telegram trading systems |

## Terminology

| Term | Meaning in this project |
|---|---|
| Diagnosis | A structured interpretation derived from transaction metadata, program identity, documented mappings, and logs |
| Evidence | Exact RPC fields or log lines that support a diagnosis |
| Confirmed classification | Program identity and documented error mapping or an explicit runtime log support the conclusion |
| Probable classification | Multiple signals support a conclusion, but the available evidence is incomplete |
| Unknown classification | The project cannot defend a human-readable cause |
| Landed failure | A failed transaction returned by `getTransaction` with execution metadata and a fee |
| Off-chain failure | A bot, API, quote, network, or application error with no landed Solana transaction |
| Telegram connector | Rendering and payload code that converts a diagnosis into a bot-ready message without receiving a bot token |

## Documentation rules

- Verify external contracts again when implementation starts.
- Pin dependency versions in `Cargo.lock` and the web lockfile.
- Record the documentation URL and verification date beside every program-specific error mapping.
- Treat RPC responses and program logs as untrusted data.
- Keep public claims narrower than the implementation evidence.
- Update these documents whenever a fixed decision changes.

## Sources verified on 24 September 2026

- Solana [`getTransaction`](https://solana.com/docs/rpc/http/gettransaction): confirmed transaction response and nullable result.
- Solana [RPC JSON structures](https://solana.com/docs/rpc/json-structures): transaction metadata, transaction errors, logs, inner instructions, balances, fees, and loaded addresses.
- Jupiter [common errors](https://dev.jup.ag/docs/swap/common-errors): program-specific error mapping and user-facing error guidance; mappings are tied to the on-chain program ID rather than assumed globally.
- Telegram [Bot API `sendMessage`](https://core.telegram.org/bots/api#sendmessage): message text, formatting, and reply markup.
- Telegram [`InlineKeyboardMarkup`](https://core.telegram.org/bots/api#inlinekeyboardmarkup): inline button rows.
- Telegram [`getUpdates`](https://core.telegram.org/bots/api#getupdates): long-polling offset and timeout behavior used by the demo bot.
