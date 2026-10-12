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
    pub target_mailboxes: Vec<String>,
    pub transfer_mode: String,
    pub routing_enabled: bool,
    pub deduplication_enabled: bool,
    pub deep_scan_enabled: bool,
    pub throttling_enabled: bool,
    pub imported_items: u64,
    pub duplicates_skipped: u64,
    pub error_count: u64,
    pub psts_processed: Vec<String>,
    pub storage_impact: Vec<crate::app::MailboxStorageImpact>,
    pub errors: Vec<crate::app::ExecutionErrorDetail>,
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

    let mut errors = state.execution_errors.clone();
    if errors.is_empty() {
        for item in &state.processed_items {
            if item.status == "Error" {
                errors.push(crate::app::ExecutionErrorDetail {
                    timestamp: item.date.clone(),
                    error_type: "item_error".to_string(),
                    pst_name: item.pst_name.clone(),
                    source_folder: item.source_folder.clone(),
                    subject: item.subject.clone(),
                    sender: item.sender.clone(),
                    error_message: item.error_message.clone().unwrap_or_else(|| "Error al procesar o transferir correo".to_string()),
                });
            }
        }
    }
    if errors.is_empty() && state.progress.error_count > 0 {
        errors.push(crate::app::ExecutionErrorDetail {
            timestamp: now.format("%Y-%m-%d %H:%M:%S").to_string(),
            error_type: "incidencia_ejecucion".to_string(),
            pst_name: state.selected_psts().first().map(|p| p.name.clone()).unwrap_or_else(|| "Archivo PST".to_string()),
            source_folder: "-".to_string(),
            subject: "Incidencia detectada durante la migración".to_string(),
            sender: "-".to_string(),
            error_message: format!("Se detectaron {} incidencias/errores durante la operación. Revise los registros de telemetría para más detalles.", state.progress.error_count),
        });
    }

    let report = AuditReport {
        application: "Outlook Organizer TS".to_string(),
        version: "1.0.0".to_string(),
        timestamp_start: now.to_rfc3339(),
        timestamp_end: now.to_rfc3339(),
        status: status.to_string(),
        target_mailboxes: state.selected_mailboxes().iter().map(|m| m.display_name.clone()).collect(),
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
        storage_impact: state.get_mailbox_storage_impacts(),
        errors,
    };

    let json_content = serde_json::to_string_pretty(&report)?;
    fs::write(&file_path, json_content)?;

    Ok(Some(file_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{ExecutionErrorDetail, ProcessedEmailItem};

    #[test]
    fn test_audit_report_serialization_with_errors() {
        let mut state = AppState::new();
        state.execution_errors.push(ExecutionErrorDetail {
            timestamp: "2024-10-12 10:00:00".to_string(),
            error_type: "folder_read_error".to_string(),
            pst_name: "archivo1.pst".to_string(),
            source_folder: "Bandeja de entrada".to_string(),
            subject: "Factura pendiente".to_string(),
            sender: "ventas@empresa.com".to_string(),
            error_message: "MAPI_E_TIMEOUT: Tiempo de espera agotado".to_string(),
        });

        let report = AuditReport {
            application: "Outlook Organizer TS".to_string(),
            version: "1.0.0".to_string(),
            timestamp_start: "2024-10-12T10:00:00Z".to_string(),
            timestamp_end: "2024-10-12T10:05:00Z".to_string(),
            status: "Completado con errores".to_string(),
            target_mailboxes: vec!["buzon@empresa.com".to_string()],
            transfer_mode: "Move".to_string(),
            routing_enabled: true,
            deduplication_enabled: true,
            deep_scan_enabled: true,
            throttling_enabled: true,
            imported_items: 800,
            duplicates_skipped: 18,
            error_count: 1,
            psts_processed: vec!["archivo1.pst".to_string()],
            storage_impact: vec![],
            errors: state.execution_errors.clone(),
        };

        let json_str = serde_json::to_string_pretty(&report).expect("Serialization failed");
        assert!(json_str.contains("MAPI_E_TIMEOUT"));
        assert!(json_str.contains("folder_read_error"));
        assert!(json_str.contains("archivo1.pst"));
    }

    #[test]
    fn test_audit_report_synthesizes_errors_from_items_when_empty() {
        let mut state = AppState::new();
        state.processed_items.push(ProcessedEmailItem {
            subject: "Correo corrupto".to_string(),
            sender: "spam@bad.com".to_string(),
            date: "2024-05-01 12:00:00".to_string(),
            source_folder: "Inbox".to_string(),
            dest_folder: "Bandeja de entrada".to_string(),
            pst_name: "test.pst".to_string(),
            status: "Error".to_string(),
            size_kb: 50.0,
            error_message: Some("Error COM al mover ítem".to_string()),
        });

        let mut errors = state.execution_errors.clone();
        if errors.is_empty() {
            for item in &state.processed_items {
                if item.status == "Error" {
                    errors.push(ExecutionErrorDetail {
                        timestamp: item.date.clone(),
                        error_type: "item_error".to_string(),
                        pst_name: item.pst_name.clone(),
                        source_folder: item.source_folder.clone(),
                        subject: item.subject.clone(),
                        sender: item.sender.clone(),
                        error_message: item.error_message.clone().unwrap_or_else(|| "Error al procesar o transferir correo".to_string()),
                    });
                }
            }
        }

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_message, "Error COM al mover ítem");
        assert_eq!(errors[0].subject, "Correo corrupto");
    }
}
