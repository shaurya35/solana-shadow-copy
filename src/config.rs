use std::{
    env, fmt,
    io::ErrorKind,
    net::{AddrParseError, SocketAddr},
};

use reqwest::Url;
use thiserror::Error;

const DEFAULT_API_BIND_ADDR: &str = "0.0.0.0:8080";

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

    fn from_lookup(mut lookup: impl FnMut(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let rpc_url = parse_http_url("SOLANA_RPC_URL", required(&mut lookup, "SOLANA_RPC_URL")?)?;
        let bind_addr = lookup("API_BIND_ADDR")
            .unwrap_or_else(|| DEFAULT_API_BIND_ADDR.to_owned())
            .parse()
            .map_err(|source| ConfigError::InvalidBindAddress {
                name: "API_BIND_ADDR",
                source,
            })?;

        Ok(Self { rpc_url, bind_addr })
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
        let telegram_bot_token = SecretString(required(&mut lookup, "TELEGRAM_BOT_TOKEN")?);
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
            ("TELEGRAM_BOT_TOKEN", "123456:secret-token"),
            ("DIAGNOSTICS_API_URL", "http://127.0.0.1:8080"),
        ]))
        .expect("valid bot configuration");

        assert_eq!(format!("{:?}", config.telegram_bot_token()), "[REDACTED]");
        assert_eq!(config.telegram_bot_token().expose(), "123456:secret-token");
    }
}
