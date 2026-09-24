# Product specification

## What it does

Solana Trade Diagnostics turns one finalized Solana transaction signature into a plain-English failure explanation, supporting evidence, and a Telegram-ready message.

It is a small integration prototype for Telegram trading workflows. It is not part of TradeWithLash and does not claim to describe a missing Lash feature.

## Why it is useful

A failed transaction can expose only an instruction number, a program address, a numeric custom code, and raw logs. The service combines those fields so a trader or support engineer can quickly see:

- what failed;
- which evidence supports that conclusion;
- whether the conclusion is confirmed or unknown;
- what fee the chain reported;
- where to inspect the transaction in Solana Explorer.

## Main flow

1. The user sends `/explain <signature>` to the demo bot.
2. The bot calls the diagnostics API.
3. The API fetches the finalized transaction from Solana RPC.
4. Deterministic rules classify the failure.
5. The bot returns a short explanation and an Explorer button.

## Supported results

| Result | When it is used | Confidence |
|---|---|---|
| Insufficient SOL | The runtime log states the available and required lamports | Confirmed |
| Jupiter slippage | The failing program is Jupiter Swap and the custom code is documented `6001` | Confirmed |
| Unmapped program error | A program and custom code exist, but this project has no verified mapping | Unknown |
| Generic transaction error | The transaction failed without enough evidence for a more specific result | Unknown |
| Transaction succeeded | Finalized metadata contains no transaction error | Confirmed |

Unknown is an intentional result. The service does not borrow an error meaning from another program or invent one.

## Version 0.1 scope

- Solana mainnet-beta.
- Finalized legacy and version `0` transactions.
- One transaction signature per request.
- Rust HTTP API.
- Telegram message renderer and long-polling demo bot.
- Three checked-in mainnet fixtures.
- Docker image and GitHub Actions checks.

## Outside the scope

- Wallet connection, private keys, signing, simulation, or transaction submission.
- Copy trading, wallet monitoring, routing, quotes, or PnL.
- Failures that happened only inside a bot or quote service and never landed on-chain.
- Automatic retry advice.
- A complete decoder for every Solana program.
- A hosted public service or production integration.

## Safety rules

- Bind every named custom error to an exact program ID and a public source.
- Show `unknown` when the evidence is not enough.
- Use integer lamports for all SOL formatting.
- Escape program-controlled text before Telegram rendering.
- Never ask for a wallet, seed phrase, private key, or bot token.
- Never tell a user that increasing slippage is automatically safe.

## Proof required for the showcase

- A real insufficient-SOL transaction produces the expected confirmed result.
- A real Jupiter `6001` transaction produces the expected confirmed result.
- A real unmapped custom error remains unknown.
- Invalid input does not call Solana RPC.
- The wrong program with error `6001` is not called Jupiter slippage.
- The HTTP API and real Telegram bot complete the same flow end to end.
- Tests, lint, release build, Docker build, and hosted CI pass.

The dated proof and screenshots are in [END_TO_END_TEST.md](./END_TO_END_TEST.md).
