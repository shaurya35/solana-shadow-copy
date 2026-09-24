use std::time::Duration;

use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

use crate::{
    config::BotConfig,
    render_telegram::{LinkPreviewOptions, TelegramMessage},
};

const LONG_POLL_SECONDS: u64 = 30;
const RETRY_DELAY: Duration = Duration::from_secs(1);

const INSUFFICIENT_SIGNATURE: &str =
    "5DLZ8sA6FPFThpKiD2QGzX3ufPjbV3hRk7Xf7P1A7m3kgB6KX6wcRcND4BdWXNwfbJxV1XNo7JooZJsj7GBrcuYn";
const JUPITER_SIGNATURE: &str =
    "5UntMZRg4ChcYbsY5orMi3ez7uvc64GdheL8R39sWiPRfCeSqQzhK9JYGQ1eeri9LhFYKDVL84KKPQamQJy7V1xR";
const UNKNOWN_SIGNATURE: &str =
    "42CkCpX9maDhJmFNZj5dDCS4uoo1ELSqyosuQp9BAU4e8xMbRx3K39DnX3UctgbeRdjukAJo9UTpwGHgKq8nZhUM";

#[derive(Clone)]
pub struct TelegramBot {
    telegram: TelegramClient,
    diagnostics: DiagnosticsClient,
}

#[derive(Clone)]
struct TelegramClient {
    http: Client,
    api_base: Url,
}

#[derive(Clone)]
struct DiagnosticsClient {
    http: Client,
    endpoint: Url,
}

#[derive(Debug, Deserialize)]
struct TelegramEnvelope<T> {
    ok: bool,
    result: Option<T>,
    error_code: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct Update {
    update_id: i64,
    message: Option<Message>,
}

#[derive(Debug, Deserialize)]
struct Message {
    chat: Chat,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Chat {
    id: i64,
}

#[derive(Debug, Deserialize)]
struct DiagnosticsResponse {
    telegram_message: Option<TelegramMessage>,
}

#[derive(Debug, Serialize)]
struct PlainMessage<'a> {
    chat_id: i64,
    text: &'a str,
    link_preview_options: LinkPreviewOptions,
}

#[derive(Debug, Error)]
pub enum TelegramError {
    #[error("failed to build an HTTP client")]
    ClientBuild,
    #[error("failed to build a service URL")]
    InvalidUrl,
    #[error("Telegram request failed")]
    TelegramRequest,
    #[error("Telegram returned an invalid response")]
    InvalidTelegramResponse,
    #[error("Telegram rejected the request with code {0}")]
    TelegramRejected(u16),
    #[error("diagnostics API request failed")]
    DiagnosticsRequest,
    #[error("diagnostics API returned HTTP status {0}")]
    DiagnosticsStatus(StatusCode),
    #[error("diagnostics API returned an invalid response")]
    InvalidDiagnosticsResponse,
}

impl TelegramBot {
    pub fn new(config: &BotConfig) -> Result<Self, TelegramError> {
        let telegram_http = Client::builder()
            .timeout(Duration::from_secs(LONG_POLL_SECONDS + 10))
            .build()
            .map_err(|_| TelegramError::ClientBuild)?;
        let diagnostics_http = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|_| TelegramError::ClientBuild)?;
        let api_base = Url::parse(&format!(
            "https://api.telegram.org/bot{}/",
            config.telegram_bot_token().expose()
        ))
        .map_err(|_| TelegramError::InvalidUrl)?;
        let endpoint = config
            .diagnostics_api_url()
            .join("v1/diagnoses")
            .map_err(|_| TelegramError::InvalidUrl)?;

        Ok(Self {
            telegram: TelegramClient {
                http: telegram_http,
                api_base,
            },
            diagnostics: DiagnosticsClient {
                http: diagnostics_http,
                endpoint,
            },
        })
    }

    pub async fn run(self) {
        let mut offset = None;

        loop {
            match self.telegram.get_updates(offset).await {
                Ok(updates) => {
                    for update in updates {
                        offset = Some(update.update_id.saturating_add(1));
                        if let Err(error) = self.handle_update(update).await {
                            tracing::warn!(%error, "failed to handle Telegram update");
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "Telegram long poll failed");
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        }
    }

    async fn handle_update(&self, update: Update) -> Result<(), TelegramError> {
        let Some(message) = update.message else {
            return Ok(());
        };
        let Some(text) = message.text.as_deref() else {
            return Ok(());
        };

        match parse_command(text) {
            Command::Start => {
                self.telegram
                    .send_text(
                        message.chat.id,
                        "Solana Trade Diagnostics explains finalized transaction failures from their signature. It is read-only and never asks for a wallet or private key.\n\nUse /examples or /explain <signature>.",
                    )
                    .await
            }
            Command::Help => {
                self.telegram
                    .send_text(
                        message.chat.id,
                        "Commands:\n/start — purpose and safety\n/examples — verified demo transactions\n/explain <signature> — diagnose one finalized mainnet transaction",
                    )
                    .await
            }
            Command::Examples => {
                self.telegram
                    .send_text(
                        message.chat.id,
                        &format!(
                            "Verified examples:\n\nInsufficient SOL:\n{INSUFFICIENT_SIGNATURE}\n\nJupiter slippage:\n{JUPITER_SIGNATURE}\n\nUnknown custom error:\n{UNKNOWN_SIGNATURE}"
                        ),
                    )
                    .await
            }
            Command::Explain(signature) => {
                if signature.is_empty() {
                    return self
                        .telegram
                        .send_text(message.chat.id, "Usage: /explain <transaction signature>")
                        .await;
                }

                match self.diagnostics.diagnose(signature).await {
                    Ok(diagnosis) => {
                        self.telegram
                            .send_diagnosis(message.chat.id, &diagnosis)
                            .await
                    }
                    Err(error) => {
                        tracing::warn!(%error, "diagnostics request failed");
                        self.telegram
                            .send_text(
                                message.chat.id,
                                "I could not diagnose that transaction. Check that it is one finalized Solana transaction signature and try again.",
                            )
                            .await
                    }
                }
            }
            Command::Unknown => {
                self.telegram
                    .send_text(message.chat.id, "Unknown command. Use /help for available commands.")
                    .await
            }
        }
    }
}

impl TelegramClient {
    async fn get_updates(&self, offset: Option<i64>) -> Result<Vec<Update>, TelegramError> {
        let url = self
            .api_base
            .join("getUpdates")
            .map_err(|_| TelegramError::InvalidUrl)?;
        let response = self
            .http
            .post(url)
            .json(&json!({
                "offset": offset,
                "timeout": LONG_POLL_SECONDS,
                "allowed_updates": ["message"]
            }))
            .send()
            .await
            .map_err(|_| TelegramError::TelegramRequest)?;
        parse_telegram_response(response).await
    }

    async fn send_text(&self, chat_id: i64, text: &str) -> Result<(), TelegramError> {
        let payload = PlainMessage {
            chat_id,
            text,
            link_preview_options: LinkPreviewOptions { is_disabled: true },
        };
        self.send_message(&payload).await
    }

    async fn send_diagnosis(
        &self,
        chat_id: i64,
        message: &TelegramMessage,
    ) -> Result<(), TelegramError> {
        let mut payload =
            serde_json::to_value(message).map_err(|_| TelegramError::InvalidTelegramResponse)?;
        payload["chat_id"] = Value::from(chat_id);
        self.send_message(&payload).await
    }

    async fn send_message(&self, payload: &impl Serialize) -> Result<(), TelegramError> {
        let url = self
            .api_base
            .join("sendMessage")
            .map_err(|_| TelegramError::InvalidUrl)?;
        let response = self
            .http
            .post(url)
            .json(payload)
            .send()
            .await
            .map_err(|_| TelegramError::TelegramRequest)?;
        let _: Value = parse_telegram_response(response).await?;
        Ok(())
    }
}

impl DiagnosticsClient {
    async fn diagnose(&self, signature: &str) -> Result<TelegramMessage, TelegramError> {
        let response = self
            .http
            .post(self.endpoint.clone())
            .json(&json!({ "signature": signature }))
            .send()
            .await
            .map_err(|_| TelegramError::DiagnosticsRequest)?;
        let status = response.status();
        if !status.is_success() {
            return Err(TelegramError::DiagnosticsStatus(status));
        }
        let response: DiagnosticsResponse = response
            .json()
            .await
            .map_err(|_| TelegramError::InvalidDiagnosticsResponse)?;
        response
            .telegram_message
            .ok_or(TelegramError::InvalidDiagnosticsResponse)
    }
}

async fn parse_telegram_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
) -> Result<T, TelegramError> {
    let envelope: TelegramEnvelope<T> = response
        .json()
        .await
        .map_err(|_| TelegramError::InvalidTelegramResponse)?;
    if !envelope.ok {
        return Err(TelegramError::TelegramRejected(
            envelope.error_code.unwrap_or(0),
        ));
    }
    envelope
        .result
        .ok_or(TelegramError::InvalidTelegramResponse)
}

#[derive(Debug, PartialEq, Eq)]
enum Command<'a> {
    Start,
    Help,
    Examples,
    Explain(&'a str),
    Unknown,
}

fn parse_command(text: &str) -> Command<'_> {
    let mut parts = text.trim().splitn(2, char::is_whitespace);
    let command = parts.next().unwrap_or_default();
    let command = command.split('@').next().unwrap_or(command);
    let argument = parts.next().map(str::trim).unwrap_or_default();

    match command {
        "/start" => Command::Start,
        "/help" => Command::Help,
        "/examples" => Command::Examples,
        "/explain" => Command::Explain(argument),
        _ => Command::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, parse_command};

    #[test]
    fn parses_supported_commands_and_bot_mentions() {
        assert_eq!(parse_command("/start"), Command::Start);
        assert_eq!(parse_command("/help"), Command::Help);
        assert_eq!(
            parse_command("/examples@diagnostics_bot"),
            Command::Examples
        );
        assert_eq!(
            parse_command("/explain  transaction-signature  "),
            Command::Explain("transaction-signature")
        );
        assert_eq!(parse_command("/explain"), Command::Explain(""));
        assert_eq!(parse_command("hello"), Command::Unknown);
    }
}
