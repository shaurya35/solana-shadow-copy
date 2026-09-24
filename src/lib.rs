//! Core library for Solana transaction diagnostics.

pub mod api;
pub mod classify;
pub mod config;
pub mod domain;
pub mod invocation_logs;
pub mod normalize;
pub mod render_telegram;
pub mod rpc;

use serde_json::Value;

pub use domain::Diagnosis;
pub use normalize::NormalizeError;

pub fn diagnose_rpc_response(response: &Value) -> Result<Diagnosis, NormalizeError> {
    let transaction = normalize::normalize_rpc_response(response)?;
    Ok(classify::classify(&transaction))
}
