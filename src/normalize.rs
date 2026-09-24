use serde_json::Value;
use thiserror::Error;

use crate::invocation_logs;

const MAX_LOG_LINES: usize = 256;
const MAX_LOG_LINE_BYTES: usize = 1_024;

#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedTransaction {
    pub signature: String,
    pub succeeded: bool,
    pub fee_lamports: Option<u64>,
    pub compute_units_consumed: Option<u64>,
    pub failed_instruction: Option<NormalizedFailedInstruction>,
    pub transaction_error: Option<Value>,
    pub log_messages: Vec<String>,
    pub attributed_failure_program_id: Option<String>,
    pub invocation_logs_malformed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedFailedInstruction {
    pub top_level_index: usize,
    pub program_id: Option<String>,
    pub custom_code: Option<u64>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum NormalizeError {
    #[error("RPC response did not contain a transaction")]
    TransactionUnavailable,
    #[error("transaction metadata is unavailable")]
    MetadataUnavailable,
    #[error("transaction version is not supported")]
    UnsupportedTransactionVersion,
    #[error("RPC response is malformed at {0}")]
    Malformed(&'static str),
}

pub fn normalize_rpc_response(response: &Value) -> Result<NormalizedTransaction, NormalizeError> {
    let result = response
        .get("result")
        .ok_or(NormalizeError::Malformed("result"))?;
    if result.is_null() {
        return Err(NormalizeError::TransactionUnavailable);
    }

    validate_version(result.get("version"))?;

    let transaction = result
        .get("transaction")
        .ok_or(NormalizeError::Malformed("result.transaction"))?;
    let signature = transaction
        .pointer("/signatures/0")
        .and_then(Value::as_str)
        .ok_or(NormalizeError::Malformed(
            "result.transaction.signatures[0]",
        ))?
        .to_owned();
    let meta = result
        .get("meta")
        .ok_or(NormalizeError::Malformed("result.meta"))?;
    if meta.is_null() {
        return Err(NormalizeError::MetadataUnavailable);
    }

    let transaction_error = meta.get("err").filter(|error| !error.is_null()).cloned();
    let succeeded = transaction_error.is_none();
    let fee_lamports = optional_u64(meta, "fee", "result.meta.fee")?;
    let compute_units_consumed = optional_u64(
        meta,
        "computeUnitsConsumed",
        "result.meta.computeUnitsConsumed",
    )?;
    let log_messages = normalize_logs(meta.get("logMessages"))?;
    let invocation_analysis = invocation_logs::analyze(&log_messages);
    let failed_instruction = match transaction_error.as_ref() {
        Some(error) => normalize_failed_instruction(error, transaction)?,
        None => None,
    };
    let attributed_failure_program_id = if invocation_analysis.malformed {
        failed_instruction
            .as_ref()
            .and_then(|instruction| instruction.program_id.clone())
    } else {
        invocation_analysis
            .deepest_failure
            .map(|failure| failure.program_id)
            .or_else(|| {
                failed_instruction
                    .as_ref()
                    .and_then(|instruction| instruction.program_id.clone())
            })
    };

    Ok(NormalizedTransaction {
        signature,
        succeeded,
        fee_lamports,
        compute_units_consumed,
        failed_instruction,
        transaction_error,
        log_messages,
        attributed_failure_program_id,
        invocation_logs_malformed: invocation_analysis.malformed,
    })
}

fn validate_version(version: Option<&Value>) -> Result<(), NormalizeError> {
    match version {
        None => Ok(()),
        Some(Value::String(value)) if value == "legacy" => Ok(()),
        Some(Value::Number(value)) if value.as_u64() == Some(0) => Ok(()),
        _ => Err(NormalizeError::UnsupportedTransactionVersion),
    }
}

fn optional_u64(
    object: &Value,
    key: &str,
    path: &'static str,
) -> Result<Option<u64>, NormalizeError> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .map(Some)
            .ok_or(NormalizeError::Malformed(path)),
    }
}

fn normalize_logs(value: Option<&Value>) -> Result<Vec<String>, NormalizeError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    if value.is_null() {
        return Ok(Vec::new());
    }

    let logs = value
        .as_array()
        .ok_or(NormalizeError::Malformed("result.meta.logMessages"))?;

    logs.iter()
        .take(MAX_LOG_LINES)
        .map(|line| {
            let line = line
                .as_str()
                .ok_or(NormalizeError::Malformed("result.meta.logMessages[]"))?;
            Ok(truncate_utf8(line, MAX_LOG_LINE_BYTES).to_owned())
        })
        .collect()
}

fn normalize_failed_instruction(
    error: &Value,
    transaction: &Value,
) -> Result<Option<NormalizedFailedInstruction>, NormalizeError> {
    let Some(instruction_error) = error.get("InstructionError") else {
        return Ok(None);
    };
    let parts = instruction_error
        .as_array()
        .filter(|parts| parts.len() == 2)
        .ok_or(NormalizeError::Malformed(
            "result.meta.err.InstructionError",
        ))?;
    let top_level_index = parts[0]
        .as_u64()
        .and_then(|index| usize::try_from(index).ok())
        .ok_or(NormalizeError::Malformed(
            "result.meta.err.InstructionError[0]",
        ))?;
    let custom_code = parts[1].get("Custom").and_then(Value::as_u64);
    let program_id = resolve_program_id(transaction, top_level_index)?;

    Ok(Some(NormalizedFailedInstruction {
        top_level_index,
        program_id,
        custom_code,
    }))
}

fn resolve_program_id(
    transaction: &Value,
    instruction_index: usize,
) -> Result<Option<String>, NormalizeError> {
    let message = transaction
        .get("message")
        .ok_or(NormalizeError::Malformed("result.transaction.message"))?;
    let instructions = message
        .get("instructions")
        .and_then(Value::as_array)
        .ok_or(NormalizeError::Malformed(
            "result.transaction.message.instructions",
        ))?;
    let instruction = instructions
        .get(instruction_index)
        .ok_or(NormalizeError::Malformed("failed instruction index"))?;

    if let Some(program_id) = instruction.get("programId").and_then(Value::as_str) {
        return Ok(Some(program_id.to_owned()));
    }

    let Some(program_id_index) = instruction.get("programIdIndex").and_then(Value::as_u64) else {
        return Ok(None);
    };
    let program_id_index = usize::try_from(program_id_index)
        .map_err(|_| NormalizeError::Malformed("programIdIndex"))?;
    let account_keys =
        message
            .get("accountKeys")
            .and_then(Value::as_array)
            .ok_or(NormalizeError::Malformed(
                "result.transaction.message.accountKeys",
            ))?;
    let account_key = account_keys
        .get(program_id_index)
        .ok_or(NormalizeError::Malformed("programIdIndex"))?;
    let program_id = account_key
        .as_str()
        .or_else(|| account_key.get("pubkey").and_then(Value::as_str))
        .ok_or(NormalizeError::Malformed("account key"))?;

    Ok(Some(program_id.to_owned()))
}

fn truncate_utf8(value: &str, max_bytes: usize) -> &str {
    if value.len() <= max_bytes {
        return value;
    }

    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

#[cfg(test)]
mod tests {
    use super::truncate_utf8;

    #[test]
    fn truncates_only_at_a_utf8_boundary() {
        assert_eq!(truncate_utf8("a💥b", 4), "a");
        assert_eq!(truncate_utf8("a💥b", 5), "a💥");
    }
}
