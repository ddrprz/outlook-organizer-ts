use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BackendMessage {
    #[serde(rename = "progress")]
    Progress {
        pst_index: usize,
        pst_total: usize,
        pst_name: String,
        item_current: u64,
        item_total: u64,
        speed_mps: f64,
        eta_seconds: u64,
    },
    #[serde(rename = "log")]
    Log {
        timestamp: String,
        level: String,
        message: String,
    },
    #[serde(rename = "throttling")]
    Throttling {
        active: bool,
        delay_ms: u64,
    },
    #[serde(rename = "finished")]
    Finished {
        status: String, // "completed", "cancelled", "failed"
        imported: u64,
        duplicates: u64,
        errors: u64,
    },
    #[serde(rename = "mailboxes_loaded")]
    MailboxesLoaded {
        items: Vec<crate::app::MailboxItem>,
    },
    #[serde(rename = "pst_detail_loaded")]
    PstDetailLoaded {
        pst_path: String,
        pst_name: String,
        detail: Result<crate::app::PstDetail, String>,
    },
}
