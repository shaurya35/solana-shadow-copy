use std::{
    env, fmt,
    io::ErrorKind,
    net::{AddrParseError, SocketAddr},
    time::Duration,
};

use reqwest::Url;
use thiserror::Error;

const DEFAULT_API_BIND_ADDR: &str = "0.0.0.0:8080";
const DEFAULT_RPC_CONNECT_TIMEOUT_MS: u64 = 2_000;
const DEFAULT_RPC_REQUEST_TIMEOUT_MS: u64 = 8_000;

#[derive(Clone)]
pub struct SecretString(String);

impl SecretString {
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

#[derive(Clone)]
pub struct ServerConfig {
    rpc_url: Url,
    bind_addr: SocketAddr,
    rpc_connect_timeout: Duration,
    rpc_request_timeout: Duration,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        load_dotenv()?;
        Self::from_lookup(|name| env::var(name).ok())
    }

    pub fn rpc_url(&self) -> &Url {
        &self.rpc_url
    }

    pub fn bind_addr(&self) -> SocketAddr {
        self.bind_addr
    }

    pub fn rpc_connect_timeout(&self) -> Duration {
        self.rpc_connect_timeout
    }

    pub fn rpc_request_timeout(&self) -> Duration {
        self.rpc_request_timeout
    }

    fn from_lookup(mut lookup: impl FnMut(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let rpc_url = parse_http_url("SOLANA_RPC_URL", required(&mut lookup, "SOLANA_RPC_URL")?)?;
        let bind_addr = lookup("API_BIND_ADDR")
            .unwrap_or_else(|| DEFAULT_API_BIND_ADDR.to_owned())
            .parse()
            .map_err(|source| ConfigError::InvalidBindAddress {
                name: "API_BIND_ADDR",
                source,
            })?;
        let rpc_connect_timeout = parse_milliseconds(
            &mut lookup,
            "RPC_CONNECT_TIMEOUT_MS",
            DEFAULT_RPC_CONNECT_TIMEOUT_MS,
        )?;
        let rpc_request_timeout = parse_milliseconds(
            &mut lookup,
            "RPC_REQUEST_TIMEOUT_MS",
            DEFAULT_RPC_REQUEST_TIMEOUT_MS,
        )?;

        Ok(Self {
            rpc_url,
            bind_addr,
            rpc_connect_timeout,
            rpc_request_timeout,
        })
    }
}

#[derive(Clone)]
pub struct BotConfig {
    telegram_bot_token: SecretString,
    diagnostics_api_url: Url,
}

impl BotConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        load_dotenv()?;
        Self::from_lookup(|name| env::var(name).ok())
    }

    pub fn telegram_bot_token(&self) -> &SecretString {
        &self.telegram_bot_token
    }

    pub fn diagnostics_api_url(&self) -> &Url {
        &self.diagnostics_api_url
    }

    fn from_lookup(mut lookup: impl FnMut(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let telegram_bot_token =
            parse_telegram_token(required(&mut lookup, "TELEGRAM_BOT_TOKEN")?)?;
        let diagnostics_api_url = parse_http_url(
            "DIAGNOSTICS_API_URL",
            required(&mut lookup, "DIAGNOSTICS_API_URL")?,
        )?;

        Ok(Self {
            telegram_bot_token,
            diagnostics_api_url,
        })
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to load .env configuration")]
    Dotenv(#[source] dotenvy::Error),
    #[error("required environment variable {0} is missing or empty")]
    Missing(&'static str),
    #[error("environment variable {name} must be a valid HTTP or HTTPS URL")]
    InvalidUrl { name: &'static str },
    #[error("environment variable {name} must be a valid socket address")]
    InvalidBindAddress {
        name: &'static str,
        #[source]
        source: AddrParseError,
    },
    #[error("environment variable {name} must be a positive integer number of milliseconds")]
    InvalidMilliseconds { name: &'static str },
    #[error("TELEGRAM_BOT_TOKEN is not a valid Telegram bot token")]
    InvalidTelegramToken,
}

fn load_dotenv() -> Result<(), ConfigError> {
    match dotenvy::dotenv() {
        Ok(_) => Ok(()),
        Err(dotenvy::Error::Io(error)) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(ConfigError::Dotenv(error)),
    }
}

fn required(
    lookup: &mut impl FnMut(&str) -> Option<String>,
    name: &'static str,
) -> Result<String, ConfigError> {
    lookup(name)
        .filter(|value| !value.trim().is_empty())
        .ok_or(ConfigError::Missing(name))
}

fn parse_http_url(name: &'static str, value: String) -> Result<Url, ConfigError> {
    let url = Url::parse(&value).map_err(|_| ConfigError::InvalidUrl { name })?;

    match url.scheme() {
        "http" | "https" => Ok(url),
        _ => Err(ConfigError::InvalidUrl { name }),
    }
}

fn parse_telegram_token(value: String) -> Result<SecretString, ConfigError> {
    let Some((bot_id, secret)) = value.split_once(':') else {
        return Err(ConfigError::InvalidTelegramToken);
    };
    let valid_bot_id = !bot_id.is_empty() && bot_id.bytes().all(|byte| byte.is_ascii_digit());
    let valid_secret = !secret.is_empty()
        && secret
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'));

    if valid_bot_id && valid_secret {
        Ok(SecretString(value))
    } else {
        Err(ConfigError::InvalidTelegramToken)
    }
}

fn parse_milliseconds(
    lookup: &mut impl FnMut(&str) -> Option<String>,
    name: &'static str,
    default: u64,
) -> Result<Duration, ConfigError> {
    let milliseconds = match lookup(name) {
        Some(value) => value
            .parse::<u64>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or(ConfigError::InvalidMilliseconds { name })?,
        None => default,
    };

    Ok(Duration::from_millis(milliseconds))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{BotConfig, ConfigError, DEFAULT_API_BIND_ADDR, ServerConfig};

    fn lookup(values: &[(&str, &str)]) -> impl FnMut(&str) -> Option<String> {
        let values: HashMap<_, _> = values
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect();

        move |name| values.get(name).cloned()
    }

    #[test]
    fn server_config_uses_default_bind_address() {
        let config = ServerConfig::from_lookup(lookup(&[(
            "SOLANA_RPC_URL",
            "https://api.mainnet-beta.solana.com",
        )]))
        .expect("valid server configuration");

        assert_eq!(config.bind_addr().to_string(), DEFAULT_API_BIND_ADDR);
        assert_eq!(config.rpc_connect_timeout().as_millis(), 2_000);
        assert_eq!(config.rpc_request_timeout().as_millis(), 8_000);
        assert_eq!(
            config.rpc_url().as_str(),
            "https://api.mainnet-beta.solana.com/"
        );
    }

    #[test]
    fn server_config_rejects_non_http_rpc_url_without_echoing_it() {
        let error = ServerConfig::from_lookup(lookup(&[(
            "SOLANA_RPC_URL",
            "file:///private/secret-rpc-key",
        )]))
        .err()
        .expect("non-HTTP URL must fail");
        let message = error.to_string();

        assert!(matches!(error, ConfigError::InvalidUrl { .. }));
        assert!(!message.contains("secret-rpc-key"));
    }

    #[test]
    fn bot_config_requires_a_token() {
        let error =
            BotConfig::from_lookup(lookup(&[("DIAGNOSTICS_API_URL", "http://127.0.0.1:8080")]))
                .err()
                .expect("missing token must fail");

        assert!(matches!(error, ConfigError::Missing("TELEGRAM_BOT_TOKEN")));
    }

    #[test]
    fn bot_token_debug_output_is_redacted() {
        let config = BotConfig::from_lookup(lookup(&[
            ("TELEGRAM_BOT_TOKEN", "123456:secret-token_123"),
            ("DIAGNOSTICS_API_URL", "http://127.0.0.1:8080"),
        ]))
        .expect("valid bot configuration");

        assert_eq!(format!("{:?}", config.telegram_bot_token()), "[REDACTED]");
        assert_eq!(
            config.telegram_bot_token().expose(),
            "123456:secret-token_123"
        );
    }

    #[test]
    fn bot_config_rejects_a_malformed_token_without_echoing_it() {
        let token = "not-a-bot-id:private-secret";
        let error = BotConfig::from_lookup(lookup(&[
            ("TELEGRAM_BOT_TOKEN", token),
            ("DIAGNOSTICS_API_URL", "http://127.0.0.1:8080"),
        ]))
        .err()
        .expect("malformed token must fail");

        assert!(matches!(error, ConfigError::InvalidTelegramToken));
        assert!(!error.to_string().contains(token));
    }
}
