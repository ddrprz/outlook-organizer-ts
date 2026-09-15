use std::{env, fs, path::PathBuf};
use chrono::Local;
use serde::Serialize;

use crate::app::AppState;

#[derive(Serialize)]
pub struct AuditReport {
    pub application: String,
    pub version: String,
    pub timestamp_start: String,
    pub timestamp_end: String,
    pub status: String,
    pub target_mailbox: String,
    pub transfer_mode: String,
    pub routing_enabled: bool,
    pub deduplication_enabled: bool,
    pub deep_scan_enabled: bool,
    pub throttling_enabled: bool,
    pub imported_items: u64,
    pub duplicates_skipped: u64,
    pub error_count: u64,
    pub psts_processed: Vec<String>,
}

/// Genera automáticamente la auditoría JSON según las directrices:
/// IMPORTANTE: Solo se genera de forma automática si la aplicación se ejecuta como .exe de producción
pub fn generate_audit_json(state: &AppState, status: &str) -> Result<Option<PathBuf>, std::io::Error> {
    // 1. Verificar si estamos corriendo en producción (.exe empaquetado) o si está forzado
    let is_release_exe = if let Ok(current_exe) = env::current_exe() {
        let path_str = current_exe.to_string_lossy().to_lowercase();
        // Si el ejecutable no está dentro de target\debug
        !path_str.contains(r"target\debug")
    } else {
        false
    };

    if !is_release_exe {
        // En modo cargo run / debug no se genera automáticamente para no ensuciar
        return Ok(None);
    }

    // 2. Crear subcarpeta fechada ./logs/YYYY-MM-DD/
    let now = Local::now();
    let date_folder = now.format("%Y-%m-%d").to_string();
    let time_str = now.format("%H-%M-%S").to_string();

    let logs_dir = PathBuf::from("logs").join(date_folder);
    fs::create_dir_all(&logs_dir)?;

    let file_path = logs_dir.join(format!("run_{}.json", time_str));

    let report = AuditReport {
        application: "Outlook Organizer TS".to_string(),
        version: "1.0.0".to_string(),
        timestamp_start: now.to_rfc3339(),
        timestamp_end: now.to_rfc3339(),
        status: status.to_string(),
        target_mailbox: state.target_mailbox.clone(),
        transfer_mode: format!("{:?}", state.transfer_mode),
        routing_enabled: state.routing_enabled,
        deduplication_enabled: state.deduplication_enabled,
        deep_scan_enabled: state.deep_scan_enabled,
        throttling_enabled: state.adaptive_throttling_enabled,
        imported_items: state.progress.imported_count,
        duplicates_skipped: state.progress.duplicates_skipped,
        error_count: state.progress.error_count,
        psts_processed: state
            .discovered_psts
            .iter()
            .filter(|p| p.selected)
            .map(|p| p.name.clone())
            .collect(),
    };

    let json_content = serde_json::to_string_pretty(&report)?;
    fs::write(&file_path, json_content)?;

    Ok(Some(file_path))
}
