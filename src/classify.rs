use crate::{
    domain::{
        CLASSIFIER_VERSION, Category, Cluster, Confidence, Diagnosis, EvidenceItem, EvidenceKind,
        EvidenceStrength, FailedInstruction, SCHEMA_VERSION, SourceMapping, TransactionStatus,
    },
    normalize::NormalizedTransaction,
};

const SYSTEM_PROGRAM_ID: &str = "11111111111111111111111111111111";
const JUPITER_PROGRAM_ID: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
const JUPITER_ERROR_SOURCE: &str = "https://dev.jup.ag/docs/swap/common-errors";
const SOLANA_JSON_SOURCE: &str = "https://solana.com/docs/rpc/json-structures";
const LOG_TAIL_LINES: usize = 12;

pub fn classify(transaction: &NormalizedTransaction) -> Diagnosis {
    if transaction.succeeded {
        return successful_diagnosis(transaction);
    }

    if let Some((line, available, required)) = transaction
        .log_messages
        .iter()
        .find_map(|line| parse_insufficient_lamports(line).map(|values| (line, values.0, values.1)))
    {
        return insufficient_balance(transaction, line, available, required);
    }

    if transaction.failure_program_from_logs
        && transaction.attributed_failure_program_id.as_deref() == Some(JUPITER_PROGRAM_ID)
        && transaction
            .failed_instruction
            .as_ref()
            .and_then(|instruction| instruction.custom_code)
            == Some(6001)
    {
        return jupiter_slippage(transaction);
    }

    if transaction
        .failed_instruction
        .as_ref()
        .and_then(|instruction| instruction.custom_code)
        .is_some()
    {
        return unknown_program_error(transaction);
    }

    unknown_transaction_error(transaction)
}

fn base_diagnosis(transaction: &NormalizedTransaction) -> Diagnosis {
    Diagnosis {
        schema_version: SCHEMA_VERSION.to_owned(),
        classifier_version: CLASSIFIER_VERSION.to_owned(),
        signature: transaction.signature.clone(),
        cluster: Cluster::MainnetBeta,
        status: TransactionStatus::Failed,
        category: Category::UnknownTransactionError,
        title: String::new(),
        explanation: String::new(),
        confidence: Confidence::Unknown,
        fee_lamports: transaction.fee_lamports.map(|fee| fee.to_string()),
        fee_sol: transaction.fee_lamports.map(format_lamports),
        compute_units_consumed: transaction
            .compute_units_consumed
            .map(|units| units.to_string()),
        failed_instruction: transaction.failed_instruction.as_ref().map(|instruction| {
            FailedInstruction {
                top_level_index: instruction.top_level_index,
                program_id: transaction
                    .attributed_failure_program_id
                    .clone()
                    .or_else(|| instruction.program_id.clone()),
                custom_code: instruction.custom_code,
            }
        }),
        evidence: common_evidence(transaction),
        next_actions: Vec::new(),
        sources: Vec::new(),
        explorer_url: format!(
            "https://explorer.solana.com/tx/{}?cluster=mainnet",
            transaction.signature
        ),
        log_tail: log_tail(&transaction.log_messages),
    }
}

fn successful_diagnosis(transaction: &NormalizedTransaction) -> Diagnosis {
    let mut diagnosis = base_diagnosis(transaction);
    diagnosis.status = TransactionStatus::Succeeded;
    diagnosis.category = Category::TransactionSucceeded;
    diagnosis.title = "Transaction succeeded".to_owned();
    diagnosis.explanation =
        "The finalized transaction metadata does not contain a transaction error.".to_owned();
    diagnosis.confidence = Confidence::Confirmed;
    diagnosis.failed_instruction = None;
    diagnosis.next_actions = vec!["No failure diagnosis is required.".to_owned()];
    diagnosis
}

fn insufficient_balance(
    transaction: &NormalizedTransaction,
    log_line: &str,
    available: u64,
    required: u64,
) -> Diagnosis {
    let mut diagnosis = base_diagnosis(transaction);
    diagnosis.category = Category::InsufficientTransferBalance;
    diagnosis.title = "Trade failed: insufficient SOL".to_owned();
    diagnosis.explanation = format!(
        "The transaction attempted to transfer {} SOL while the runtime reported {} SOL available.",
        format_lamports(required),
        format_lamports(available)
    );
    diagnosis.confidence = Confidence::Confirmed;
    diagnosis.evidence.insert(
        0,
        EvidenceItem {
            kind: EvidenceKind::RuntimeLog,
            value: log_line.to_owned(),
            source_path: "result.meta.logMessages".to_owned(),
            strength: EvidenceStrength::Primary,
            redacted: false,
        },
    );
    diagnosis.next_actions = vec![
        "Reduce the requested amount or ensure the wallet has enough SOL for the amount, fees, and account requirements."
            .to_owned(),
        "Request a fresh transaction instead of resubmitting the same signed transaction."
            .to_owned(),
    ];
    diagnosis.sources = vec![SourceMapping {
        program_id: SYSTEM_PROGRAM_ID.to_owned(),
        code: "runtime-log".to_owned(),
        name: "insufficient lamports".to_owned(),
        source_url: SOLANA_JSON_SOURCE.to_owned(),
        verified_at: "2026-09-24".to_owned(),
    }];
    diagnosis
}

fn jupiter_slippage(transaction: &NormalizedTransaction) -> Diagnosis {
    let mut diagnosis = base_diagnosis(transaction);
    diagnosis.category = Category::JupiterSlippageToleranceExceeded;
    diagnosis.title = "Trade failed: slippage tolerance exceeded".to_owned();
    diagnosis.explanation = "The verified Jupiter Swap program returned custom error 6001, documented as SlippageToleranceExceeded. The executable price crossed the transaction's allowed threshold."
        .to_owned();
    diagnosis.confidence = Confidence::Confirmed;
    diagnosis.next_actions = vec![
        "Request a fresh quote and review its price and slippage before signing another transaction."
            .to_owned(),
        "Only increase slippage after considering the additional execution risk.".to_owned(),
    ];
    diagnosis.sources = vec![SourceMapping {
        program_id: JUPITER_PROGRAM_ID.to_owned(),
        code: "6001".to_owned(),
        name: "SlippageToleranceExceeded".to_owned(),
        source_url: JUPITER_ERROR_SOURCE.to_owned(),
        verified_at: "2026-09-24".to_owned(),
    }];
    diagnosis
}

fn unknown_program_error(transaction: &NormalizedTransaction) -> Diagnosis {
    let mut diagnosis = base_diagnosis(transaction);
    let instruction = transaction
        .failed_instruction
        .as_ref()
        .expect("caller checked custom instruction error");
    let program_id = transaction
        .attributed_failure_program_id
        .as_deref()
        .or(instruction.program_id.as_deref())
        .unwrap_or("unknown program");
    let code = instruction
        .custom_code
        .expect("caller checked custom instruction error");

    diagnosis.category = Category::UnknownProgramError;
    diagnosis.title = "Transaction failed with an unmapped program error".to_owned();
    diagnosis.explanation = format!(
        "Program {program_id} returned custom error {code}. This service has no verified meaning for that program-and-code pair."
    );
    diagnosis.next_actions = vec![
        "Review the program's verified documentation or contact the transaction provider with this signature."
            .to_owned(),
    ];
    diagnosis
}

fn unknown_transaction_error(transaction: &NormalizedTransaction) -> Diagnosis {
    let mut diagnosis = base_diagnosis(transaction);
    diagnosis.title = "Transaction failed with an unsupported error".to_owned();
    diagnosis.explanation =
        "The transaction failed, but the available evidence does not match a verified classifier."
            .to_owned();
    diagnosis.next_actions =
        vec!["Review the bounded log tail and the transaction in Solana Explorer.".to_owned()];
    diagnosis
}

fn common_evidence(transaction: &NormalizedTransaction) -> Vec<EvidenceItem> {
    let mut evidence = Vec::new();

    if let Some(instruction) = &transaction.failed_instruction {
        evidence.push(EvidenceItem {
            kind: EvidenceKind::InstructionError,
            value: match instruction.custom_code {
                Some(code) => format!(
                    "instruction {} returned custom error {}",
                    instruction.top_level_index, code
                ),
                None => format!("instruction {} failed", instruction.top_level_index),
            },
            source_path: "result.meta.err.InstructionError".to_owned(),
            strength: EvidenceStrength::Supporting,
            redacted: false,
        });
    } else if let Some(error) = &transaction.transaction_error {
        evidence.push(EvidenceItem {
            kind: EvidenceKind::TransactionError,
            value: bounded_text(&error.to_string(), 512),
            source_path: "result.meta.err".to_owned(),
            strength: EvidenceStrength::Primary,
            redacted: false,
        });
    }

    if let Some(program_id) = &transaction.attributed_failure_program_id {
        evidence.push(EvidenceItem {
            kind: EvidenceKind::ProgramId,
            value: program_id.clone(),
            source_path: if transaction.failure_program_from_logs {
                "result.meta.logMessages".to_owned()
            } else {
                "result.transaction.message.instructions".to_owned()
            },
            strength: EvidenceStrength::Supporting,
            redacted: false,
        });
    }

    if let Some(fee) = transaction.fee_lamports {
        evidence.push(EvidenceItem {
            kind: EvidenceKind::Fee,
            value: fee.to_string(),
            source_path: "result.meta.fee".to_owned(),
            strength: EvidenceStrength::Supporting,
            redacted: false,
        });
    }

    evidence
}

fn parse_insufficient_lamports(line: &str) -> Option<(u64, u64)> {
    let amounts = line.strip_prefix("Transfer: insufficient lamports ")?;
    let (available, required) = amounts.split_once(", need ")?;
    Some((available.parse().ok()?, required.parse().ok()?))
}

pub fn format_lamports(lamports: u64) -> String {
    const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

    let whole = lamports / LAMPORTS_PER_SOL;
    let remainder = lamports % LAMPORTS_PER_SOL;
    if remainder == 0 {
        return whole.to_string();
    }

    let fraction = format!("{remainder:09}");
    format!("{whole}.{}", fraction.trim_end_matches('0'))
}

fn log_tail(logs: &[String]) -> Vec<String> {
    logs.iter()
        .skip(logs.len().saturating_sub(LOG_TAIL_LINES))
        .cloned()
        .collect()
}

fn bounded_text(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }

    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

#[cfg(test)]
mod tests {
    use super::{format_lamports, parse_insufficient_lamports};

    #[test]
    fn formats_lamports_without_floating_point() {
        assert_eq!(format_lamports(0), "0");
        assert_eq!(format_lamports(1), "0.000000001");
        assert_eq!(format_lamports(55_000), "0.000055");
        assert_eq!(format_lamports(2_100_979_360), "2.10097936");
        assert_eq!(format_lamports(2_500_000_000), "2.5");
    }

    #[test]
    fn accepts_only_the_exact_insufficient_lamports_grammar() {
        assert_eq!(
            parse_insufficient_lamports(
                "Transfer: insufficient lamports 2100979360, need 2500000000"
            ),
            Some((2_100_979_360, 2_500_000_000))
        );
        assert_eq!(
            parse_insufficient_lamports("insufficient lamports 1, need 2"),
            None
        );
    }
}
