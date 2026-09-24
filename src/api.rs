use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, FromRequest, Request, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{
    Diagnosis, NormalizeError, diagnose_rpc_response,
    domain::Category,
    render_telegram::{self, TelegramMessage},
    rpc::{RpcClient, RpcError, ValidatedSignature, validate_signature},
};

const MAX_REQUEST_BYTES: usize = 4 * 1024;
static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

type TransactionFuture<'a> = Pin<Box<dyn Future<Output = Result<Value, RpcError>> + Send + 'a>>;

trait TransactionSource: Send + Sync {
    fn get_transaction<'a>(&'a self, signature: &'a ValidatedSignature) -> TransactionFuture<'a>;
}

impl TransactionSource for RpcClient {
    fn get_transaction<'a>(&'a self, signature: &'a ValidatedSignature) -> TransactionFuture<'a> {
        Box::pin(async move { self.get_transaction(signature).await })
    }
}

#[derive(Clone)]
struct AppState {
    source: Arc<dyn TransactionSource>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosisRequest {
    signature: String,
    #[serde(default = "default_cluster")]
    cluster: String,
    #[serde(default)]
    include: IncludeOptions,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct IncludeOptions {
    telegram_message: bool,
    log_tail: bool,
}

impl Default for IncludeOptions {
    fn default() -> Self {
        Self {
            telegram_message: true,
            log_tail: true,
        }
    }
}

#[derive(Debug, Serialize)]
struct DiagnosisResponse {
    request_id: String,
    #[serde(flatten)]
    diagnosis: Diagnosis,
    #[serde(skip_serializing_if = "Option::is_none")]
    telegram_message: Option<TelegramMessage>,
    analyzed_at: String,
}

#[derive(Debug, Serialize)]
struct Example {
    signature: &'static str,
    expected_category: Category,
    label: &'static str,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    request_id: String,
}

pub fn router(rpc: RpcClient) -> Router {
    router_with_source(Arc::new(rpc))
}

fn router_with_source(source: Arc<dyn TransactionSource>) -> Router {
    Router::new()
        .route("/v1/diagnoses", post(diagnose))
        .route("/v1/examples", get(examples))
        .route("/health/live", get(health))
        .route("/health/ready", get(health))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .with_state(AppState { source })
}

async fn diagnose(State(state): State<AppState>, request: Request<Body>) -> Response {
    let request_id = request_id(request.headers());
    let payload = match Json::<DiagnosisRequest>::from_request(request, &state).await {
        Ok(Json(payload)) => payload,
        Err(error) => {
            return ApiError::invalid_json(request_id, error).into_response();
        }
    };

    match diagnose_inner(&state, payload, &request_id).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => error.into_response(),
    }
}

async fn diagnose_inner(
    state: &AppState,
    payload: DiagnosisRequest,
    request_id: &str,
) -> Result<DiagnosisResponse, ApiError> {
    if payload.cluster != "mainnet-beta" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "only cluster mainnet-beta is supported",
            request_id,
        ));
    }

    let signature = validate_signature(&payload.signature).map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_signature",
            "signature must be one base58-encoded Solana transaction signature",
            request_id,
        )
    })?;
    let response = state
        .source
        .get_transaction(&signature)
        .await
        .map_err(|error| ApiError::from_rpc(error, request_id))?;
    let mut diagnosis = diagnose_rpc_response(&response)
        .map_err(|error| ApiError::from_normalize(error, request_id))?;
    if !payload.include.log_tail {
        diagnosis.log_tail.clear();
    }
    let telegram_message = payload
        .include
        .telegram_message
        .then(|| render_telegram::render(&diagnosis));

    Ok(DiagnosisResponse {
        request_id: request_id.to_owned(),
        diagnosis,
        telegram_message,
        analyzed_at: analyzed_at(),
    })
}

async fn examples() -> Json<Vec<Example>> {
    Json(vec![
        Example {
            signature: "5DLZ8sA6FPFThpKiD2QGzX3ufPjbV3hRk7Xf7P1A7m3kgB6KX6wcRcND4BdWXNwfbJxV1XNo7JooZJsj7GBrcuYn",
            expected_category: Category::InsufficientTransferBalance,
            label: "Insufficient SOL transfer",
        },
        Example {
            signature: "5UntMZRg4ChcYbsY5orMi3ez7uvc64GdheL8R39sWiPRfCeSqQzhK9JYGQ1eeri9LhFYKDVL84KKPQamQJy7V1xR",
            expected_category: Category::JupiterSlippageToleranceExceeded,
            label: "Jupiter slippage tolerance exceeded",
        },
        Example {
            signature: "42CkCpX9maDhJmFNZj5dDCS4uoo1ELSqyosuQp9BAU4e8xMbRx3K39DnX3UctgbeRdjukAJo9UTpwGHgKq8nZhUM",
            expected_category: Category::UnknownProgramError,
            label: "Unknown custom program error",
        },
    ])
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

impl ApiError {
    fn new(
        status: StatusCode,
        code: &'static str,
        message: &'static str,
        request_id: &str,
    ) -> Self {
        Self {
            status,
            code,
            message,
            request_id: request_id.to_owned(),
        }
    }

    fn invalid_json(request_id: String, _error: JsonRejection) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "request body must match the diagnosis JSON contract",
            &request_id,
        )
    }

    fn from_rpc(error: RpcError, request_id: &str) -> Self {
        match error {
            RpcError::RateLimited => Self::new(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "RPC provider rate limited the request",
                request_id,
            ),
            RpcError::Transport(_) | RpcError::HttpStatus(_) => Self::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "rpc_unavailable",
                "RPC provider is temporarily unavailable",
                request_id,
            ),
            RpcError::InvalidSignature => Self::new(
                StatusCode::BAD_REQUEST,
                "invalid_signature",
                "signature must be one base58-encoded Solana transaction signature",
                request_id,
            ),
            RpcError::ClientBuild(_)
            | RpcError::ResponseTooLarge
            | RpcError::InvalidJson(_)
            | RpcError::MismatchedResponseId
            | RpcError::JsonRpc { .. } => Self::new(
                StatusCode::BAD_GATEWAY,
                "rpc_invalid_response",
                "RPC provider returned an invalid response",
                request_id,
            ),
        }
    }

    fn from_normalize(error: NormalizeError, request_id: &str) -> Self {
        match error {
            NormalizeError::TransactionUnavailable => Self::new(
                StatusCode::NOT_FOUND,
                "transaction_unavailable",
                "transaction was not found at finalized commitment",
                request_id,
            ),
            NormalizeError::MetadataUnavailable => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "metadata_unavailable",
                "transaction metadata is unavailable",
                request_id,
            ),
            NormalizeError::UnsupportedTransactionVersion => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "unsupported_transaction_version",
                "transaction version is not supported",
                request_id,
            ),
            NormalizeError::Malformed(_) => Self::new(
                StatusCode::BAD_GATEWAY,
                "rpc_invalid_response",
                "RPC provider returned an invalid response",
                request_id,
            ),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({
                "error": {
                    "code": self.code,
                    "message": self.message,
                    "request_id": self.request_id,
                }
            })),
        )
            .into_response()
    }
}

fn default_cluster() -> String {
    "mainnet-beta".to_owned()
}

fn request_id(headers: &HeaderMap) -> String {
    if let Some(value) = headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        })
    {
        return value.to_owned();
    }

    let sequence = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("req-{nanos:x}-{sequence:x}")
}

fn analyzed_at() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".to_owned())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use super::*;

    #[derive(Clone)]
    struct FixtureSource {
        response: Value,
        calls: Arc<AtomicUsize>,
    }

    impl TransactionSource for FixtureSource {
        fn get_transaction<'a>(
            &'a self,
            _signature: &'a ValidatedSignature,
        ) -> TransactionFuture<'a> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            let response = self.response.clone();
            Box::pin(async move { Ok(response) })
        }
    }

    fn fixture_router(response: Value, calls: Arc<AtomicUsize>) -> Router {
        router_with_source(Arc::new(FixtureSource { response, calls }))
    }

    #[tokio::test]
    async fn invalid_signature_does_not_call_rpc() {
        let calls = Arc::new(AtomicUsize::new(0));
        let app = fixture_router(Value::Null, calls.clone());
        let response = app
            .oneshot(
                Request::post("/v1/diagnoses")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"signature":"wallet-address"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(calls.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn returns_fixture_backed_diagnosis_and_telegram_message() {
        let fixture: Value =
            serde_json::from_str(include_str!("../fixtures/insufficient_transfer.json")).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let app = fixture_router(fixture, calls.clone());
        let response = app
            .oneshot(
                Request::post("/v1/diagnoses")
                    .header("content-type", "application/json")
                    .header("x-request-id", "test-request")
                    .body(Body::from(
                        json!({ "signature": "5DLZ8sA6FPFThpKiD2QGzX3ufPjbV3hRk7Xf7P1A7m3kgB6KX6wcRcND4BdWXNwfbJxV1XNo7JooZJsj7GBrcuYn" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["request_id"], "test-request");
        assert_eq!(body["category"], "insufficient_transfer_balance");
        assert_eq!(body["confidence"], "confirmed");
        assert_eq!(body["telegram_message"]["parse_mode"], "HTML");
    }
}
