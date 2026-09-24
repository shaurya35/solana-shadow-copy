# API and Telegram

## Routes

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/v1/diagnoses` | Diagnose one finalized transaction |
| `GET` | `/v1/examples` | List the three verified demo signatures |
| `GET` | `/health/live` | Confirm the API process is running |
| `GET` | `/health/ready` | Confirm the API process is running |

The health routes do not call Solana RPC.

## Diagnose a transaction

Request:

```http
POST /v1/diagnoses
Content-Type: application/json
```

```json
{
  "signature": "5DLZ8sA6FPFThpKiD2QGzX3ufPjbV3hRk7Xf7P1A7m3kgB6KX6wcRcND4BdWXNwfbJxV1XNo7JooZJsj7GBrcuYn",
  "cluster": "mainnet-beta",
  "include": {
    "telegram_message": true,
    "log_tail": true
  }
}
```

Only `signature` is required. `cluster` defaults to `mainnet-beta`, and both `include` values default to `true`. Unknown JSON fields are rejected.

The response contains:

- `status`, `category`, `title`, `explanation`, and `confidence`;
- the fee and compute units when RPC returned them;
- the failed instruction, attributed program, and custom code when known;
- evidence with its source path;
- safe next steps and documentation sources;
- an Explorer URL and bounded log tail;
- a Telegram message object when requested;
- a request ID and analysis time.

Confirmed and unknown diagnoses use the same response shape.

## Errors

Errors use this shape:

```json
{
  "error": {
    "code": "invalid_signature",
    "message": "signature must be one base58-encoded Solana transaction signature",
    "request_id": "req-..."
  }
}
```

| Status | Meaning |
|---:|---|
| `400` | Invalid JSON, signature, cluster, or request fields |
| `404` | No finalized transaction was found |
| `422` | Metadata is missing or the transaction version is unsupported |
| `429` | The RPC provider rate-limited the request |
| `502` | The RPC response was invalid |
| `503` | The RPC provider could not be reached |

Callers may send `X-Request-Id`. The API accepts up to 64 ASCII letters, digits, hyphens, or underscores; otherwise it creates its own ID.

## Telegram message

The API can return a `telegram_message` object with:

- escaped HTML text;
- `parse_mode: HTML`;
- disabled link previews;
- one `View transaction` inline button.

It never returns a bot token or `chat_id`. The calling bot supplies those values.

## Demo bot

Commands:

| Command | Result |
|---|---|
| `/start` | Explain the tool and its read-only boundary |
| `/help` | List commands |
| `/examples` | Show the three verified signatures |
| `/explain <signature>` | Call the diagnostics API and send its result |

The bot validates the token format at startup and calls Telegram `getMe` before logging that it has started. It uses `getUpdates` long polling and retries polling failures after one second.

## Existing-bot integration

An existing Telegram bot needs only this flow:

```text
failed transaction signature
    -> POST /v1/diagnoses
    -> add the bot's chat_id to telegram_message
    -> Telegram sendMessage
```

The prototype proves this boundary with its own demo bot. Integrating it into another company's bot still requires access to that codebase, deployment, and product decisions.

## Public deployment notes

The code limits request bodies and global concurrency. A public deployment should add authentication or gateway rate limiting before accepting untrusted traffic at scale.
