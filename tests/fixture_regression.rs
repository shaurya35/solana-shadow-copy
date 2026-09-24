use serde_json::Value;

const SIGNATURE: &str =
    "5DLZ8sA6FPFThpKiD2QGzX3ufPjbV3hRk7Xf7P1A7m3kgB6KX6wcRcND4BdWXNwfbJxV1XNo7JooZJsj7GBrcuYn";
const SYSTEM_PROGRAM_ID: &str = "11111111111111111111111111111111";
const INSUFFICIENT_LAMPORTS_LOG: &str =
    "Transfer: insufficient lamports 2100979360, need 2500000000";

fn insufficient_transfer_fixture() -> Value {
    serde_json::from_str(include_str!("../fixtures/insufficient_transfer.json"))
        .expect("insufficient-transfer fixture must contain valid JSON")
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
}
