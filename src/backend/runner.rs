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
    #[serde(default)]
    pub selected_folder_paths: Vec<String>,
    pub routing_enabled: bool,
    pub routing_granularity: String,
    pub specific_year: Option<u32>,
    pub specific_month: Option<u32>,
    #[serde(default)]
    pub specific_years: Vec<u32>,
    #[serde(default)]
    pub specific_months: Vec<u32>,
    pub deduplication_enabled: bool,
    pub deep_scan_enabled: bool,
    pub adaptive_throttling: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SplitWorkerConfig {
    pub profile_name: Option<String>,
    pub source_pst_path: String,
    #[serde(default)]
    pub source_pst_paths: Vec<String>,
    pub output_dir: String,
    pub partition_mode: String,
    pub transfer_mode: String,
    pub selected_years: Vec<u32>,
    pub selected_months: Vec<u32>,
    pub include_inbox: bool,
    pub include_sent: bool,
    pub include_deleted: bool,
    pub include_custom_folders: bool,
    pub adaptive_throttling: bool,
}

/// Ejecuta el worker de PowerShell de forma completamente asíncrona sin bloquear la UI
pub struct BackendRunner;

#[cfg(windows)]
fn configure_low_overhead_command(cmd: &mut Command) {
    const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x00004000;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    cmd.creation_flags(BELOW_NORMAL_PRIORITY_CLASS | CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn configure_low_overhead_command(_cmd: &mut Command) {}

impl BackendRunner {
    pub fn spawn_worker(
        config: &WorkerConfig,
        tx: UnboundedSender<BackendMessage>,
    ) -> Result<(tokio::process::Child, PathBuf, PathBuf), std::io::Error> {
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join("outlook_organizer_worker.ps1");
        let config_path = temp_dir.join("outlook_organizer_config.json");
        let abort_path = temp_dir.join("outlook_organizer_abort.flag");
        let pause_path = temp_dir.join("outlook_organizer_pause.flag");

        // Limpiar bandera previa de cancelación e historial previo de items
        if abort_path.exists() {
            let _ = fs::remove_file(&abort_path);
        }
        if pause_path.exists() {
            let _ = fs::remove_file(&pause_path);
        }
        let items_path = temp_dir.join("outlook_organizer_items.json");
        if items_path.exists() {
            let _ = fs::remove_file(&items_path);
        }

        // Escribir script y archivo de configuración JSON con BOM UTF-8 para PowerShell 5.1
        let script_content = include_str!("worker.ps1");
        fs::write(&script_path, format!("\u{feff}{}", script_content))?;

        let config_json = serde_json::to_string_pretty(config)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&config_path, config_json)?;

        let mut cmd = Command::new("powershell");
        configure_low_overhead_command(&mut cmd);
        let mut child = cmd
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
            .arg("-PauseFile")
            .arg(&pause_path)
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

        Ok((child, abort_path, pause_path))
    }

    pub fn spawn_split_worker(
        config: &SplitWorkerConfig,
        tx: UnboundedSender<BackendMessage>,
    ) -> Result<(tokio::process::Child, PathBuf, PathBuf), std::io::Error> {
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join("outlook_organizer_split_worker.ps1");
        let config_path = temp_dir.join("outlook_organizer_split_config.json");
        let abort_path = temp_dir.join("outlook_organizer_split_abort.flag");
        let pause_path = temp_dir.join("outlook_organizer_split_pause.flag");

        if abort_path.exists() {
            let _ = fs::remove_file(&abort_path);
        }
        if pause_path.exists() {
            let _ = fs::remove_file(&pause_path);
        }

        let script_content = include_str!("split_worker.ps1");
        fs::write(&script_path, format!("\u{feff}{}", script_content))?;

        let config_json = serde_json::to_string_pretty(config)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&config_path, config_json)?;

        let mut cmd = Command::new("powershell");
        configure_low_overhead_command(&mut cmd);
        let mut child = cmd
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
            .arg("-PauseFile")
            .arg(&pause_path)
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let tx_out = tx.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut buf = Vec::new();

            loop {
                buf.clear();
                match reader.read_until(b'\n', &mut buf).await {
                    Ok(0) => break,
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
                                message: format!("[PowerShell Split] {}", line),
                            });
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok((child, abort_path, pause_path))
    }

    /// Obtiene de forma asíncrona la lista de buzones configurados en Outlook MAPI
    pub async fn fetch_outlook_mailboxes(profile: Option<&str>) -> Vec<crate::app::MailboxItem> {
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join("outlook_organizer_discovery.ps1");
        let script_content = include_str!("mailbox_discovery.ps1");
        let _ = fs::write(&script_path, format!("\u{feff}{}", script_content));

        let mut cmd = Command::new("powershell");
        configure_low_overhead_command(&mut cmd);
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
                if let Ok(mut items) = serde_json::from_str::<Vec<crate::app::MailboxItem>>(stdout_str.trim()) {
                    // Garantizar que ningún archivo PST sea tratado como buzón de destino
                    items.retain(|m| {
                        m.store_type != "PST"
                            && !m.display_name.to_lowercase().ends_with(".pst")
                            && !m
                                .file_path
                                .as_deref()
                                .unwrap_or("")
                                .to_lowercase()
                                .ends_with(".pst")
                    });

                    if !items.is_empty() {
                        if !items.iter().any(|m| m.selected) {
                            items[0].selected = true;
                        }
                        return items;
                    }
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

    /// Dispara la inspección MAPI detallada de un PST en segundo plano
    pub fn trigger_pst_inspection(
        pst_path: String,
        pst_name: String,
        profile: Option<String>,
        tx: UnboundedSender<BackendMessage>,
    ) {
        tokio::spawn(async move {
            let res = Self::inspect_pst(&pst_path, profile.as_deref(), Some(&tx)).await;
            let _ = tx.send(BackendMessage::PstDetailLoaded {
                pst_path,
                pst_name,
                detail: res.map(Box::new),
            });
        });
    }

    pub async fn inspect_pst(
        pst_path: &str,
        profile: Option<&str>,
        tx: Option<&UnboundedSender<BackendMessage>>,
    ) -> Result<crate::app::PstDetail, String> {
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join("outlook_organizer_inspector.ps1");
        let script_content = include_str!("pst_inspector.ps1");
        let _ = fs::write(&script_path, format!("\u{feff}{}", script_content));

        let mut cmd = Command::new("powershell");
        configure_low_overhead_command(&mut cmd);
        cmd.arg("-Sta")
            .arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(&script_path)
            .arg("-PstPath")
            .arg(pst_path);

        if let Some(prof) = profile
            && !prof.trim().is_empty() {
            cmd.arg("-ProfileName").arg(prof);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => return Err(format!("No se pudo ejecutar PowerShell: {}", e)),
        };

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let mut last_json_line = String::new();

        if let Some(out) = stdout {
            let reader = BufReader::new(out);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Some(rest) = trimmed.strip_prefix("PROGRESS:") {
                    #[derive(serde::Deserialize)]
                    struct ProgressPayload {
                        folder: Option<String>,
                        items: Option<usize>,
                    }
                    if let Ok(p) = serde_json::from_str::<ProgressPayload>(rest)
                        && let Some(sender) = tx {
                        let _ = sender.send(BackendMessage::PstInspectionProgress {
                            pst_path: pst_path.to_string(),
                            folder_name: p.folder.unwrap_or_default(),
                            scanned_items: p.items.unwrap_or(0),
                        });
                    }
                } else {
                    last_json_line = trimmed.to_string();
                }
            }
        }

        let status = child.wait().await;
        match status {
            Ok(s) if s.success() => {
                #[derive(serde::Deserialize)]
                struct InspectorError {
                    error: Option<String>,
                }
                if let Ok(err_obj) = serde_json::from_str::<InspectorError>(&last_json_line)
                    && let Some(err_msg) = err_obj.error {
                    return Err(err_msg);
                }

                serde_json::from_str::<crate::app::PstDetail>(&last_json_line)
                    .map_err(|e| format!("Error al decodificar metadatos: {} (Salida: {})", e, last_json_line))
            }
            Ok(_) => {
                let mut err_msg = String::new();
                if let Some(err) = stderr {
                    let mut r = BufReader::new(err);
                    let mut buf = String::new();
                    let _ = tokio::io::AsyncReadExt::read_to_string(&mut r, &mut buf).await;
                    err_msg = buf;
                }
                Err(format!("Error en subproceso PowerShell: {}", err_msg))
            }
            Err(e) => Err(format!("Error al esperar finalización de PowerShell: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Requiere Outlook en ejecución interactiva y perfil configurado"]
    async fn test_fetch_outlook_mailboxes_live() {
        let items = BackendRunner::fetch_outlook_mailboxes(None).await;
        println!("FETCHED ITEMS: {:?}", items);
        assert!(!items.is_empty());
        for item in &items {
            assert_ne!(item.store_type, "PST");
            assert!(!item.display_name.to_lowercase().ends_with(".pst"));
            if let Some(ref path) = item.file_path {
                assert!(!path.to_lowercase().ends_with(".pst"));
            }
        }
    }
}
