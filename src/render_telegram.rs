use serde::{Deserialize, Serialize};

use crate::domain::{Confidence, Diagnosis};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelegramMessage {
    pub text: String,
    pub parse_mode: String,
    pub link_preview_options: LinkPreviewOptions,
    pub reply_markup: InlineKeyboardMarkup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkPreviewOptions {
    pub is_disabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineKeyboardMarkup {
    pub inline_keyboard: Vec<Vec<InlineKeyboardButton>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineKeyboardButton {
    pub text: String,
    pub url: String,
}

pub fn render(diagnosis: &Diagnosis) -> TelegramMessage {
    let confidence = match diagnosis.confidence {
        Confidence::Confirmed => "Confirmed",
        Confidence::Unknown => "Unknown",
    };

    let mut sections = vec![
        format!("<b>{}</b>", escape_html(&diagnosis.title)),
        escape_html(&diagnosis.explanation),
    ];

    if let Some(fee_sol) = &diagnosis.fee_sol {
        sections.push(format!(
            "Fee charged: <code>{} SOL</code>",
            escape_html(fee_sol)
        ));
    }
    sections.push(format!("Confidence: <b>{confidence}</b>"));

    if let Some(evidence) = diagnosis.evidence.first() {
        sections.push(format!(
            "Evidence: <code>{}</code>",
            escape_html(&evidence.value)
        ));
    }
    if let Some(action) = diagnosis.next_actions.first() {
        sections.push(format!("Next: {}", escape_html(action)));
    }

    TelegramMessage {
        text: sections.join("\n\n"),
        parse_mode: "HTML".to_owned(),
        link_preview_options: LinkPreviewOptions { is_disabled: true },
        reply_markup: InlineKeyboardMarkup {
            inline_keyboard: vec![vec![InlineKeyboardButton {
                text: "View transaction".to_owned(),
                url: diagnosis.explorer_url.clone(),
            }]],
        },
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use crate::domain::{
        Category, Cluster, Confidence, Diagnosis, EvidenceItem, EvidenceKind, EvidenceStrength,
        TransactionStatus,
    };

    use super::render;

    #[test]
    fn escapes_chain_controlled_text() {
        let diagnosis = Diagnosis {
            schema_version: "1".to_owned(),
            classifier_version: "test".to_owned(),
            signature: "signature".to_owned(),
            cluster: Cluster::MainnetBeta,
            status: TransactionStatus::Failed,
            category: Category::UnknownProgramError,
            title: "Failed <again>".to_owned(),
            explanation: "Program A&B failed".to_owned(),
            confidence: Confidence::Unknown,
            fee_lamports: None,
            fee_sol: None,
            compute_units_consumed: None,
            failed_instruction: None,
            evidence: vec![EvidenceItem {
                kind: EvidenceKind::RuntimeLog,
                value: "<unsafe>".to_owned(),
                source_path: "test".to_owned(),
                strength: EvidenceStrength::Primary,
                redacted: false,
            }],
            next_actions: vec![],
            sources: vec![],
            explorer_url: "https://explorer.solana.com/tx/signature".to_owned(),
            log_tail: vec![],
        };

        let message = render(&diagnosis);
        assert!(message.text.contains("Failed &lt;again&gt;"));
        assert!(message.text.contains("Program A&amp;B failed"));
        assert!(message.text.contains("&lt;unsafe&gt;"));
        assert!(!message.text.contains("<unsafe>"));
    }
}
