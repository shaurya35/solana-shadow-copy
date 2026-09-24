use serde::Serialize;

use crate::domain::Category;

pub const INSUFFICIENT_SIGNATURE: &str =
    "5DLZ8sA6FPFThpKiD2QGzX3ufPjbV3hRk7Xf7P1A7m3kgB6KX6wcRcND4BdWXNwfbJxV1XNo7JooZJsj7GBrcuYn";
pub const JUPITER_SIGNATURE: &str =
    "5UntMZRg4ChcYbsY5orMi3ez7uvc64GdheL8R39sWiPRfCeSqQzhK9JYGQ1eeri9LhFYKDVL84KKPQamQJy7V1xR";
pub const UNKNOWN_SIGNATURE: &str =
    "42CkCpX9maDhJmFNZj5dDCS4uoo1ELSqyosuQp9BAU4e8xMbRx3K39DnX3UctgbeRdjukAJo9UTpwGHgKq8nZhUM";

#[derive(Debug, Clone, Serialize)]
pub struct Example {
    pub signature: &'static str,
    pub expected_category: Category,
    pub label: &'static str,
}

pub fn all() -> Vec<Example> {
    vec![
        Example {
            signature: INSUFFICIENT_SIGNATURE,
            expected_category: Category::InsufficientTransferBalance,
            label: "Insufficient SOL transfer",
        },
        Example {
            signature: JUPITER_SIGNATURE,
            expected_category: Category::JupiterSlippageToleranceExceeded,
            label: "Jupiter slippage tolerance exceeded",
        },
        Example {
            signature: UNKNOWN_SIGNATURE,
            expected_category: Category::UnknownProgramError,
            label: "Unknown custom program error",
        },
    ]
}
