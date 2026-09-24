use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use reqwest::{Client, StatusCode, Url};
use serde_json::{Value, json};
use thiserror::Error;

use crate::config::ServerConfig;

const MAX_RPC_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const RETRY_DELAY: Duration = Duration::from_millis(50);

#[derive(Clone)]
pub struct RpcClient {
    http: Client,
    endpoint: Url,
    next_id: Arc<AtomicU64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedSignature(String);

impl ValidatedSignature {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Error)]
pub enum RpcError {
    #[error("signature must be one base58-encoded 64-byte Solana signature")]
    InvalidSignature,
    #[error("failed to construct the RPC HTTP client")]
    ClientBuild(#[source] reqwest::Error),
    #[error("RPC request failed")]
    Transport(#[source] reqwest::Error),
    #[error("RPC provider rate limited the request")]
    RateLimited,
    #[error("RPC provider returned HTTP status {0}")]
    HttpStatus(StatusCode),
    #[error("RPC response exceeded the configured size limit")]
    ResponseTooLarge,
    #[error("RPC provider returned malformed JSON")]
    InvalidJson(#[source] serde_json::Error),
    #[error("RPC response ID did not match the request")]
    MismatchedResponseId,
    #[error("RPC provider returned error {code}")]
    JsonRpc { code: i64 },
}

impl RpcClient {
    pub fn new(config: &ServerConfig) -> Result<Self, RpcError> {
        let http = Client::builder()
            .connect_timeout(config.rpc_connect_timeout())
            .timeout(config.rpc_request_timeout())
            .build()
            .map_err(RpcError::ClientBuild)?;

        Ok(Self {
            http,
            endpoint: config.rpc_url().clone(),
            next_id: Arc::new(AtomicU64::new(1)),
        })
    }

    pub async fn get_transaction(&self, signature: &ValidatedSignature) -> Result<Value, RpcError> {
        let request_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let body = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "getTransaction",
            "params": [
                signature.as_str(),
                {
                    "commitment": "finalized",
                    "encoding": "jsonParsed",
                    "maxSupportedTransactionVersion": 0
                }
            ]
        });

        match self
            .http
            .post(self.endpoint.clone())
            .json(&body)
            .send()
            .await
        {
            Ok(response) if !response.status().is_server_error() => {
                return parse_response(response, request_id).await;
            }
            Ok(_) | Err(_) => tokio::time::sleep(RETRY_DELAY).await,
        }

        let response = self
            .http
            .post(self.endpoint.clone())
            .json(&body)
            .send()
            .await
            .map_err(RpcError::Transport)?;
        parse_response(response, request_id).await
    }
}

pub fn validate_signature(input: &str) -> Result<ValidatedSignature, RpcError> {
    let signature = input.trim();
    if signature.is_empty() || signature.len() > 100 {
        return Err(RpcError::InvalidSignature);
    }

    let mut decoded = [0_u8; 64];
    let decoded_length = bs58::decode(signature)
        .onto(&mut decoded)
        .map_err(|_| RpcError::InvalidSignature)?;
    if decoded_length != decoded.len() {
        return Err(RpcError::InvalidSignature);
    }

    Ok(ValidatedSignature(signature.to_owned()))
}

async fn parse_response(response: reqwest::Response, request_id: u64) -> Result<Value, RpcError> {
    let status = response.status();
    if status == StatusCode::TOO_MANY_REQUESTS {
        return Err(RpcError::RateLimited);
    }
    if !status.is_success() {
        return Err(RpcError::HttpStatus(status));
    }

    let bytes = read_bounded(response).await?;
    let value: Value = serde_json::from_slice(&bytes).map_err(RpcError::InvalidJson)?;

    if value.get("id").and_then(Value::as_u64) != Some(request_id) {
        return Err(RpcError::MismatchedResponseId);
    }
    if let Some(error) = value.get("error") {
        let code = error.get("code").and_then(Value::as_i64).unwrap_or(-1);
        return Err(RpcError::JsonRpc { code });
    }

    Ok(value)
}

async fn read_bounded(mut response: reqwest::Response) -> Result<Vec<u8>, RpcError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RPC_RESPONSE_BYTES as u64)
    {
        return Err(RpcError::ResponseTooLarge);
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(RpcError::Transport)? {
        if bytes.len().saturating_add(chunk.len()) > MAX_RPC_RESPONSE_BYTES {
            return Err(RpcError::ResponseTooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::validate_signature;

    const SIGNATURE: &str =
        "5DLZ8sA6FPFThpKiD2QGzX3ufPjbV3hRk7Xf7P1A7m3kgB6KX6wcRcND4BdWXNwfbJxV1XNo7JooZJsj7GBrcuYn";

    #[test]
    fn accepts_a_sol_transaction_signature_and_trims_its_edges() {
        let signature = validate_signature(&format!("  {SIGNATURE}\n")).expect("valid signature");
        assert_eq!(signature.as_str(), SIGNATURE);
    }

    #[test]
    fn rejects_wallets_urls_and_malformed_base58() {
        assert!(validate_signature("BZgDcKUsVSCFP3kGP2uw4orurY1HSJwfzm4co1Agdt35").is_err());
        assert!(validate_signature("https://explorer.solana.com/tx/example").is_err());
        assert!(validate_signature("not-a-base58-signature").is_err());
        assert!(validate_signature(&"1".repeat(101)).is_err());
    }
}
