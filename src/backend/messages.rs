use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeneratedPstInfo {
    pub file_path: String,
    pub file_name: String,
    pub items_count: usize,
    pub size_mb: f64,
}

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
        #[serde(default)]
        imported: u64,
        #[serde(default)]
        duplicates: u64,
        #[serde(default)]
        errors: u64,
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
    #[serde(rename = "split_finished")]
    SplitFinished {
        status: String,
        total_items: u64,
        #[serde(default)]
        generated_psts: Vec<GeneratedPstInfo>,
        #[serde(default)]
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
        detail: Result<Box<crate::app::PstDetail>, String>,
    },
    #[serde(rename = "pst_inspection_progress")]
    PstInspectionProgress {
        pst_path: String,
        folder_name: String,
        scanned_items: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_message_deserialization_with_metrics() {
        let json = r#"{
            "type": "progress",
            "pst_index": 1,
            "pst_total": 2,
            "pst_name": "backup.pst",
            "item_current": 50,
            "item_total": 100,
            "speed_mps": 10.5,
            "eta_seconds": 5,
            "imported": 45,
            "duplicates": 4,
            "errors": 1
        }"#;

        let msg: BackendMessage = serde_json::from_str(json).expect("Failed to parse progress message");
        if let BackendMessage::Progress {
            imported,
            duplicates,
            errors,
            item_current,
            ..
        } = msg {
            assert_eq!(item_current, 50);
            assert_eq!(imported, 45);
            assert_eq!(duplicates, 4);
            assert_eq!(errors, 1);
        } else {
            panic!("Expected BackendMessage::Progress");
        }
    }

    #[test]
    fn test_progress_message_deserialization_defaults() {
        let json = r#"{
            "type": "progress",
            "pst_index": 1,
            "pst_total": 1,
            "pst_name": "test.pst",
            "item_current": 10,
            "item_total": 20,
            "speed_mps": 5.0,
            "eta_seconds": 2
        }"#;

        let msg: BackendMessage = serde_json::from_str(json).expect("Failed to parse progress message");
        if let BackendMessage::Progress {
            imported,
            duplicates,
            errors,
            ..
        } = msg {
            assert_eq!(imported, 0);
            assert_eq!(duplicates, 0);
            assert_eq!(errors, 0);
        } else {
            panic!("Expected BackendMessage::Progress");
        }
    }

    #[test]
    fn test_split_finished_deserialization() {
        let json = r#"{
            "type": "split_finished",
            "status": "completed",
            "total_items": 150,
            "generated_psts": [
                {
                    "file_path": "C:\\Correo\\backup_2023.pst",
                    "file_name": "backup_2023.pst",
                    "items_count": 90,
                    "size_mb": 14.5
                },
                {
                    "file_path": "C:\\Correo\\backup_2024.pst",
                    "file_name": "backup_2024.pst",
                    "items_count": 60,
                    "size_mb": 9.2
                }
            ],
            "errors": 0
        }"#;

        let msg: BackendMessage = serde_json::from_str(json).expect("Failed to parse split_finished message");
        if let BackendMessage::SplitFinished {
            status,
            total_items,
            generated_psts,
            errors,
        } = msg {
            assert_eq!(status, "completed");
            assert_eq!(total_items, 150);
            assert_eq!(generated_psts.len(), 2);
            assert_eq!(generated_psts[0].file_name, "backup_2023.pst");
            assert_eq!(generated_psts[0].items_count, 90);
            assert_eq!(generated_psts[1].file_name, "backup_2024.pst");
            assert_eq!(generated_psts[1].items_count, 60);
            assert_eq!(errors, 0);
        } else {
            panic!("Expected BackendMessage::SplitFinished");
        }
    }
}

