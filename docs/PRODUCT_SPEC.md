# Product specification

## Product statement

- Product: Solana Trade Diagnostics.
- Core job: explain why a landed Solana transaction failed.
- Primary interface: transaction signature in; structured diagnosis out.
- Primary presentation: concise Telegram message with an Explorer button.
- Secondary presentation: minimal web page for a reviewer who does not want to open Telegram.

## Project goals

- Demonstrate correct Solana transaction and log interpretation.
- Provide a reusable diagnostics component for a Telegram trading workflow.
- Handle ambiguous on-chain evidence conservatively.
- Keep the API independent from private trading logic and Telegram credentials.

## Problem evidence

- Failed trading transactions can still charge the fee payer.
- Raw Solana errors can contain:
  - Generic transaction error variants.
  - A top-level instruction index.
  - A custom numeric error without a readable name.
  - Nested cross-program invocations.
  - Logs containing the most useful explanation.
- Trading users should not need to manually correlate all of those fields.
- Generic advice such as “increase fees or slippage” can be wrong and unsafe.
- A bot already knows the failed signature and can request a diagnosis after finalization.

## Target users

| User | Need | Version `0.1` result |
|---|---|---|
| Telegram trader | Understand a failed transaction quickly | Short explanation, fee, confidence, evidence, and Explorer link |
| Trading-bot engineer | Add failure feedback without embedding classification logic in bot handlers | Stable HTTP response plus Telegram-ready render model |
| Support engineer | Triage user reports consistently | Machine-readable category, failed instruction, program, logs, and request ID |
| Technical reviewer | Verify that the conclusion comes from real Solana data | Real mainnet fixtures, source links, tests, and reproducible demo |

## Primary user flow

1. User or bot supplies a transaction signature.
2. Service validates the signature locally.
3. Service requests the finalized transaction from Solana RPC.
4. Service validates the RPC response shape.
5. Service identifies the top-level failed instruction.
6. Service reconstructs the relevant program invocation context from instructions and logs.
7. Classifiers evaluate evidence in deterministic priority order.
8. Service returns one diagnosis with confidence and supporting evidence.
9. Telegram adapter renders the diagnosis into a safe message.
10. Caller supplies its own `chat_id` and sends the message with its own bot token.

## Example result: explicit insufficient balance

- Category: `insufficient_transfer_balance`.
- Title: `Trade failed: insufficient SOL`.
- Known values:
  - Attempted transfer: `2.5 SOL`.
  - Available lamports reported by the runtime: `2.10097936 SOL`.
  - Fee charged: `0.000055 SOL`.
- Evidence:
  - `Transfer: insufficient lamports 2100979360, need 2500000000`.
- Confidence: `confirmed`.
- Safe next action:
  - Reduce the requested amount or add enough SOL for the amount plus fees and account requirements.
- Forbidden wording:
  - `Deposit exactly 0.39902064 SOL and retry`.
  - Reason: the next attempt may have different fees, rent, route, and account requirements.

## Example result: documented Jupiter slippage

- Category: `jupiter_slippage_tolerance_exceeded`.
- Title: `Trade failed: price moved beyond the allowed slippage`.
- Required evidence:
  - Failed program is the mapped Jupiter swap program.
  - Custom code is `6001`.
  - Mapping source is stored with the classifier.
- Confidence: `confirmed` only when program identity and code both match.
- Safe next action:
  - Refresh the quote and review price movement, liquidity, and configured tolerance before retrying.
- Forbidden wording:
  - `Increase slippage and retry`.
  - Reason: increasing tolerance can expose the user to a materially worse execution.

## Example result: unknown custom error

- Category: `unknown_program_error`.
- Title: `Trade failed inside an unclassified program`.
- Included details:
  - Program address.
  - Top-level instruction index.
  - Custom numeric code.
  - Fee charged.
  - Relevant log tail.
  - Explorer link.
- Confidence: `unknown`.
- Safe next action:
  - Review the program documentation or contact the transaction provider with the signature.
- Forbidden behavior:
  - Reusing a numeric mapping from a different program.
  - Generating a cause with an LLM.

## Version `0.1` functional scope

### Required

- Validate one mainnet transaction signature.
- Fetch finalized transaction data.
- Support legacy and version `0` transactions returned by the chosen RPC.
- Read transaction metadata:
  - `err`.
  - `fee`.
  - `logMessages`.
  - `computeUnitsConsumed` when present.
  - Account keys, including loaded addresses returned by parsed encoding.
  - Top-level instructions.
- Identify the failed top-level instruction index when provided.
- Attribute the strongest defensible failing-program candidate.
- Produce exactly one primary diagnosis.
- Preserve alternate evidence without presenting multiple competing causes to the user.
- Render a Telegram-safe message.
- Provide a stable JSON API.
- Provide a working demo bot.
- Provide a minimal web demo with preloaded examples.
- Include real sanitized mainnet fixtures.
- Include unit, integration, contract, and smoke tests.

### Initial classifier set

| Priority | Category | Minimum evidence |
|---:|---|---|
| 1 | `insufficient_transfer_balance` | Explicit runtime log containing available and required lamports |
| 2 | `jupiter_slippage_tolerance_exceeded` | Mapped Jupiter program ID plus documented custom code `6001` |
| 3 | `unknown_program_error` | Instruction failure with program/code but no verified mapping |
| 4 | `unknown_transaction_error` | Failed metadata that no earlier classifier can defend |

### Required non-failure states

- `invalid_signature`.
- `transaction_unavailable`.
- `transaction_succeeded`.
- `metadata_unavailable`.
- `provider_rate_limited`.
- `provider_timeout`.
- `provider_error`.

## Non-goals

- No explanation of off-chain bot errors without a transaction signature.
- No diagnosis of off-chain service failures without a landed transaction signature.
- No wallet analysis, wallet monitoring, copy trading, or trade execution.
- No fill prediction, PnL, or profitability claims.
- No recommendation to buy, sell, deposit, or raise slippage automatically.
- No transaction construction, simulation, signing, submission, or retry.
- No private key, seed phrase, wallet connection, or bot token collection.
- No universal decoder for every Solana program.
- No dynamic LLM-generated diagnosis.
- No claim that a program-specific code is stable across unrelated program IDs or upgrades.
- No production integration with a third-party system without explicit access and authorization.

## Product quality requirements

- Result must be understandable in under 15 seconds.
- Every confirmed or probable diagnosis must show its evidence.
- Unknown must be a first-class successful analysis result.
- Raw logs must be escaped before HTML or Telegram rendering.
- Fee must be labeled as the transaction fee reported by RPC.
- SOL display must derive from integer lamports without floating-point calculations.
- Program-specific mappings must include source URL and verification date.
- One transaction signature must always produce the same result for a fixed classifier version and fixture.
- The public demo must work from preloaded fixtures if the RPC is temporarily unavailable.

## Success criteria

### Engineering

- Two real mainnet failures are correctly classified from reproducible fixtures.
- One real custom error remains deliberately unknown.
- Incorrect program IDs cannot trigger Jupiter error mappings.
- The failed instruction index resolves to the correct top-level instruction.
- Malformed logs cannot crash or inject markup into the API, bot, or page.
- The API and Telegram renderer share the same domain diagnosis.

### Product

- A reviewer can paste a supported signature and understand the result.
- A reviewer can open the supporting Explorer transaction in one tap.
- The bot response states confidence and avoids unsafe retry instructions.
- The page and bot state that no wallet access or transaction execution occurs.

### Demo artifact

- Public repository has a focused README and architecture diagram.
- Hosted web example remains usable without Telegram.
- Demo bot handles `/explain`, `/examples`, `/help`, and invalid input.
- A 60–90 second recording demonstrates one confirmed and one unknown result.

## Final red-team constraints

| Dismissal risk | Required mitigation |
|---|---|
| “This is only an RPC wrapper” | Program attribution, deterministic classifiers, evidence model, confidence, fixtures, and tests |
| “Explorers already show logs” | Telegram-ready explanation and integration contract |
| “This duplicates an explorer” | Show Telegram-ready explanations, program attribution, and a stable integration contract |
| “This explains off-chain outages” | Explicitly state that off-chain failures are unsupported |
| “The explanation could cause another loss” | Conservative next actions and no automatic retry advice |
| “Custom codes are ambiguous” | Bind every code mapping to a specific program ID and source |
| “Public RPC is unreliable” | Timeouts, rate limits, bounded cache, provider errors, and fixture-backed demo |
