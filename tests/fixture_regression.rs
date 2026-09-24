use serde_json::Value;
use solana_trade_diagnostics::{
    diagnose_rpc_response,
    domain::{Category, Confidence},
};

const SIGNATURE: &str =
    "5DLZ8sA6FPFThpKiD2QGzX3ufPjbV3hRk7Xf7P1A7m3kgB6KX6wcRcND4BdWXNwfbJxV1XNo7JooZJsj7GBrcuYn";
const SYSTEM_PROGRAM_ID: &str = "11111111111111111111111111111111";
const INSUFFICIENT_LAMPORTS_LOG: &str =
    "Transfer: insufficient lamports 2100979360, need 2500000000";
const JUPITER_PROGRAM_ID: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
const UNKNOWN_PROGRAM_ID: &str = "CApP1caNy2LgFLV4WZtSf3WPaECpm6gN3zT9kCmXN29y";

fn insufficient_transfer_fixture() -> Value {
    serde_json::from_str(include_str!("../fixtures/insufficient_transfer.json"))
        .expect("insufficient-transfer fixture must contain valid JSON")
}

fn fixture(contents: &str) -> Value {
    serde_json::from_str(contents).expect("fixture must contain valid JSON")
}

#[test]
fn insufficient_transfer_fixture_contains_verified_failure_evidence() {
    let fixture = insufficient_transfer_fixture();
    let result = fixture
        .get("result")
        .expect("fixture must contain a transaction result");
    let meta = result
        .get("meta")
        .expect("fixture must contain transaction metadata");

    assert_eq!(fixture["jsonrpc"], "2.0");
    assert_eq!(result["version"], 0);
    assert_eq!(result["transaction"]["signatures"][0], SIGNATURE);
    assert_eq!(meta["err"]["InstructionError"][0], 3);
    assert_eq!(meta["err"]["InstructionError"][1]["Custom"], 1);
    assert_eq!(meta["fee"], 55_000);
    assert_eq!(meta["computeUnitsConsumed"], 16_863);

    let failed_instruction = &result["transaction"]["message"]["instructions"][3];
    assert_eq!(failed_instruction["programId"], SYSTEM_PROGRAM_ID);
    assert_eq!(failed_instruction["parsed"]["type"], "transfer");
    assert_eq!(
        failed_instruction["parsed"]["info"]["lamports"],
        2_500_000_000_u64
    );

    let logs = meta["logMessages"]
        .as_array()
        .expect("fixture metadata must contain log messages");
    assert!(
        logs.iter()
            .any(|line| line.as_str() == Some(INSUFFICIENT_LAMPORTS_LOG))
    );

    let diagnosis = diagnose_rpc_response(&fixture).expect("fixture must normalize");
    assert_eq!(diagnosis.category, Category::InsufficientTransferBalance);
    assert_eq!(diagnosis.confidence, Confidence::Confirmed);
    assert_eq!(diagnosis.fee_sol.as_deref(), Some("0.000055"));
    assert!(diagnosis.explanation.contains("2.5 SOL"));
    assert!(diagnosis.explanation.contains("2.10097936 SOL"));
}

#[test]
fn jupiter_fixture_contains_program_bound_error_6001() {
    let fixture = fixture(include_str!("../fixtures/jupiter_slippage_6001.json"));
    let result = &fixture["result"];
    let meta = &result["meta"];

    assert_eq!(result["version"], 0);
    assert_eq!(meta["err"]["InstructionError"][0], 4);
    assert_eq!(meta["err"]["InstructionError"][1]["Custom"], 6001);
    assert_eq!(meta["fee"], 8_444);
    assert_eq!(meta["computeUnitsConsumed"], 170_187);
    assert_eq!(
        result["transaction"]["message"]["instructions"][4]["programId"],
        JUPITER_PROGRAM_ID
    );
    assert!(meta["logMessages"].as_array().unwrap().iter().any(|line| {
        line.as_str()
            == Some(
                "Program JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4 failed: custom program error: 0x1771",
            )
    }));

    let diagnosis = diagnose_rpc_response(&fixture).expect("fixture must normalize");
    assert_eq!(
        diagnosis.category,
        Category::JupiterSlippageToleranceExceeded
    );
    assert_eq!(diagnosis.confidence, Confidence::Confirmed);
}

#[test]
fn unknown_fixture_preserves_unmapped_program_error() {
    let fixture = fixture(include_str!("../fixtures/unknown_custom_error.json"));
    let result = &fixture["result"];
    let meta = &result["meta"];

    assert_eq!(result["version"], 0);
    assert_eq!(meta["err"]["InstructionError"][0], 7);
    assert_eq!(meta["err"]["InstructionError"][1]["Custom"], 43_008);
    assert_eq!(meta["fee"], 5_000);
    assert_eq!(meta["computeUnitsConsumed"], 151_715);
    assert_eq!(
        result["transaction"]["message"]["instructions"][7]["programId"],
        UNKNOWN_PROGRAM_ID
    );
    assert!(meta["logMessages"].as_array().unwrap().iter().any(|line| {
        line.as_str()
            == Some(
                "Program CApP1caNy2LgFLV4WZtSf3WPaECpm6gN3zT9kCmXN29y failed: custom program error: 0xa800",
            )
    }));

    let diagnosis = diagnose_rpc_response(&fixture).expect("fixture must normalize");
    assert_eq!(diagnosis.category, Category::UnknownProgramError);
    assert_eq!(diagnosis.confidence, Confidence::Unknown);
}

#[test]
fn error_6001_from_another_program_is_not_called_jupiter_slippage() {
    let mut fixture = fixture(include_str!("../fixtures/jupiter_slippage_6001.json"));
    fixture["result"]["transaction"]["message"]["instructions"][4]["programId"] =
        Value::String(UNKNOWN_PROGRAM_ID.to_owned());
    let logs = fixture["result"]["meta"]["logMessages"]
        .as_array_mut()
        .expect("fixture log messages");
    for line in logs {
        if let Some(value) = line.as_str() {
            *line = Value::String(value.replace(JUPITER_PROGRAM_ID, UNKNOWN_PROGRAM_ID));
        }
    }

    let diagnosis = diagnose_rpc_response(&fixture).expect("fixture must normalize");
    assert_eq!(diagnosis.category, Category::UnknownProgramError);
    assert_eq!(diagnosis.confidence, Confidence::Unknown);
}
