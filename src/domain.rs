use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: &str = "1";
pub const CLASSIFIER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Cluster {
    MainnetBeta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    Failed,
    Succeeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    InsufficientTransferBalance,
    JupiterSlippageToleranceExceeded,
    UnknownProgramError,
    UnknownTransactionError,
    TransactionSucceeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Confirmed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedInstruction {
    pub top_level_index: usize,
    pub program_id: Option<String>,
    pub custom_code: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    TransactionError,
    InstructionError,
    ProgramId,
    RuntimeLog,
    Fee,
    ComputeUnits,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStrength {
    Primary,
    Supporting,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub kind: EvidenceKind,
    pub value: String,
    pub source_path: String,
    pub strength: EvidenceStrength,
    pub redacted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMapping {
    pub program_id: String,
    pub code: String,
    pub name: String,
    pub source_url: String,
    pub verified_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnosis {
    pub schema_version: String,
    pub classifier_version: String,
    pub signature: String,
    pub cluster: Cluster,
    pub status: TransactionStatus,
    pub category: Category,
    pub title: String,
    pub explanation: String,
    pub confidence: Confidence,
    pub fee_lamports: Option<String>,
    pub fee_sol: Option<String>,
    pub compute_units_consumed: Option<String>,
    pub failed_instruction: Option<FailedInstruction>,
    pub evidence: Vec<EvidenceItem>,
    pub next_actions: Vec<String>,
    pub sources: Vec<SourceMapping>,
    pub explorer_url: String,
    pub log_tail: Vec<String>,
}
