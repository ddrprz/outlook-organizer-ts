use std::{
    fs,
    path::PathBuf,
    process::Stdio,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::mpsc::UnboundedSender,
};

use super::messages::BackendMessage;

#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkerConfig {
    pub profile_name: Option<String>,
    pub psts: Vec<String>,
    pub target_mailboxes: Vec<String>,
    pub transfer_mode: String,
    pub include_inbox: bool,
    pub include_sent: bool,
    pub include_deleted: bool,
    pub include_custom_folders: bool,
    pub routing_enabled: bool,
    pub routing_granularity: String,
    pub specific_year: Option<u32>,
    pub specific_month: Option<u32>,
    pub deduplication_enabled: bool,
    pub deep_scan_enabled: bool,
    pub adaptive_throttling: bool,
}

/// Ejecuta el worker de PowerShell de forma completamente asíncrona sin bloquear la UI
pub struct BackendRunner;

impl BackendRunner {
    pub fn spawn_worker(
        config: &WorkerConfig,
        tx: UnboundedSender<BackendMessage>,
    ) -> Result<(tokio::process::Child, PathBuf), std::io::Error> {
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join("outlook_organizer_worker.ps1");
        let config_path = temp_dir.join("outlook_organizer_config.json");
        let abort_path = temp_dir.join("outlook_organizer_abort.flag");

        // Limpiar bandera previa de cancelación
        if abort_path.exists() {
            let _ = fs::remove_file(&abort_path);
        }

        // Escribir script y archivo de configuración JSON
        let script_content = include_str!("worker.ps1");
        fs::write(&script_path, script_content)?;

        let config_json = serde_json::to_string_pretty(config)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&config_path, config_json)?;

        let mut child = Command::new("powershell")
            .arg("-Sta")
            .arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(&script_path)
            .arg("-ConfigFile")
            .arg(&config_path)
            .arg("-AbortFile")
            .arg(&abort_path)
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let tx_out = tx.clone();

        // Tarea asíncrona para telemetría stdout con lectura resiliente ante codificaciones (UTF-8 lossy)
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut buf = Vec::new();

            loop {
                buf.clear();
                match reader.read_until(b'\n', &mut buf).await {
                    Ok(0) => break, // Fin del flujo (EOF)
                    Ok(_) => {
                        let line = String::from_utf8_lossy(&buf).trim().to_string();
                        if line.is_empty() {
                            continue;
                        }

                        if let Ok(msg) = serde_json::from_str::<BackendMessage>(&line) {
                            let _ = tx_out.send(msg);
                        } else {
                            let _ = tx_out.send(BackendMessage::Log {
                                timestamp: "LIVE".to_string(),
                                level: "INFO".to_string(),
                                message: line,
                            });
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Tarea asíncrona para drenar stderr y registrar advertencias sin bloquear
        let stderr = child.stderr.take().expect("Failed to capture stderr");
        let tx_err = tx.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut buf = Vec::new();

            loop {
                buf.clear();
                match reader.read_until(b'\n', &mut buf).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let line = String::from_utf8_lossy(&buf).trim().to_string();
                        if !line.is_empty() {
                            let _ = tx_err.send(BackendMessage::Log {
                                timestamp: "WARN".to_string(),
                                level: "WARN".to_string(),
                                message: format!("[PowerShell] {}", line),
                            });
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok((child, abort_path))
    }

    /// Obtiene de forma asíncrona la lista de buzones configurados en Outlook MAPI
    pub async fn fetch_outlook_mailboxes(profile: Option<&str>) -> Vec<crate::app::MailboxItem> {
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join("outlook_organizer_discovery.ps1");
        let script_content = include_str!("mailbox_discovery.ps1");
        let _ = fs::write(&script_path, script_content);

        let mut cmd = Command::new("powershell");
        cmd.arg("-Sta")
            .arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(&script_path);

        if let Some(prof) = profile
            && !prof.trim().is_empty() {
            cmd.arg("-ProfileName").arg(prof);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        match cmd.output().await {
            Ok(output) if output.status.success() => {
                let stdout_str = String::from_utf8_lossy(&output.stdout);
                if let Ok(mut items) = serde_json::from_str::<Vec<crate::app::MailboxItem>>(stdout_str.trim())
                    && !items.is_empty() {
                    if !items.iter().any(|m| m.selected) {
                        items[0].selected = true;
                    }
                    return items;
                }
                crate::app::default_fallback_mailboxes()
            }
            _ => crate::app::default_fallback_mailboxes(),
        }
    }

    /// Dispara la detección en segundo plano y notifica por canal mpsc
    pub fn trigger_mailbox_discovery(
        profile: Option<String>,
        tx: UnboundedSender<BackendMessage>,
    ) {
        tokio::spawn(async move {
            let items = Self::fetch_outlook_mailboxes(profile.as_deref()).await;
            let _ = tx.send(BackendMessage::MailboxesLoaded { items });
        });
    }
}
