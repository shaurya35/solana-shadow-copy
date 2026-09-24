# Architecture

## Data flow

```text
transaction signature
        |
        v
HTTP API -> Solana getTransaction(finalized)
        |
        v
normalize transaction, error, fee, instructions, and logs
        |
        v
attribute the failed program and classify the failure
        |
        +-> structured JSON
        +-> Telegram-safe message
```

The Telegram bot runs as a separate process. It sends the signature to the HTTP API and then sends the returned message to Telegram. The diagnostics API never receives the Telegram token or chat ID.

## Code map

| File | Job |
|---|---|
| `src/config.rs` | Load and validate environment settings without printing secrets |
| `src/rpc.rs` | Validate signatures and call Solana RPC |
| `src/normalize.rs` | Turn RPC JSON into a small internal transaction model |
| `src/invocation_logs.rs` | Follow nested program calls and find the deepest logged failure |
| `src/classify.rs` | Produce one deterministic diagnosis |
| `src/domain.rs` | Define the response types |
| `src/examples.rs` | Store the three verified public examples |
| `src/render_telegram.rs` | Escape and format Telegram HTML |
| `src/api.rs` | Expose the HTTP routes |
| `src/telegram.rs` | Poll Telegram, parse commands, and call the API |

## Trust boundaries

The transaction signature, RPC response, program logs, request ID, and Telegram messages are untrusted input.

The service therefore:

- checks that the input decodes to a 64-byte Solana signature;
- accepts only mainnet-beta, legacy, and version `0` transactions;
- limits request bodies, concurrent HTTP requests, RPC response bytes, log count, and log length;
- checks that the RPC response contains the requested signature;
- escapes all chain-controlled text before using Telegram HTML;
- keeps RPC URLs and Telegram tokens out of errors and structured output.

## Failure attribution

Solana reports the failed top-level instruction in `meta.err`. A transaction can also make nested cross-program calls. The log parser follows `invoke`, `success`, and `failed` lines as a stack and records the deepest clear failure.

If the log stack is malformed, the service does not trust it. It falls back to the program on the failed top-level instruction.

## Classifier order

The first matching rule wins:

1. An exact `Transfer: insufficient lamports <available>, need <required>` runtime log.
2. Jupiter Swap program `JUP6...` with custom error `6001` and a verified failure log.
3. A custom program error with no verified mapping.
4. A generic transaction failure.

A numeric custom error has meaning only for its program. Error `6001` from another program stays unknown.

## Confidence

- `confirmed`: an explicit runtime message or a verified program-and-code mapping supports the result.
- `unknown`: the service preserves the facts but does not name a cause it cannot prove.

There is no LLM in the diagnosis path.

## RPC behavior

The client calls `getTransaction` with:

- `commitment: finalized`;
- `encoding: jsonParsed`;
- `maxSupportedTransactionVersion: 0`.

It uses connect and request timeouts. It retries once after a transport error or server-side HTTP error. It does not retry rate limits or invalid responses.

## Runtime limits

Version `0.1` has:

- a 4 KiB request-body limit;
- at most 32 concurrent HTTP requests;
- a 2 MiB RPC-response limit;
- at most 256 stored log lines and 1,024 bytes per line;
- one RPC provider and no cache or database.

These are safe demo defaults, not a claim of production scale.

## Deployment

The Docker image contains two non-root binaries:

- `server` for the API;
- `telegram_bot` for the demo bot.

For a private integration, an existing bot can call the API and keep ownership of its Telegram credentials and user context.
