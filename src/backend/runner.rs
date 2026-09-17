use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::mpsc::UnboundedSender,
};

use super::messages::BackendMessage;

/// Ejecuta el worker de PowerShell de forma completamente asíncrona sin bloquear la UI
pub struct BackendRunner;

impl BackendRunner {
    pub fn spawn_worker(
        tx: UnboundedSender<BackendMessage>,
    ) -> Result<tokio::process::Child, std::io::Error> {
        let script_content = include_str!("worker.ps1");

        let mut child = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-Command")
            .arg(script_content)
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().expect("Failed to capture stdout");

        // Tarea asíncrona dedicada a consumir telemetría JSON Lines
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }

                // Intentar deserializar mensaje estructurado
                if let Ok(msg) = serde_json::from_str::<BackendMessage>(&line) {
                    let _ = tx.send(msg);
                } else {
                    // Fallback a log crudo
                    let _ = tx.send(BackendMessage::Log {
                        timestamp: "LIVE".to_string(),
                        level: "INFO".to_string(),
                        message: line,
                    });
                }
            }
        });

        Ok(child)
    }

    /// Obtiene de forma asíncrona la lista de buzones configurados en Outlook MAPI
    pub async fn fetch_outlook_mailboxes(profile: Option<&str>) -> Vec<crate::app::MailboxItem> {
        let script_content = include_str!("mailbox_discovery.ps1");
        let mut cmd = Command::new("powershell");
        cmd.arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-Command")
            .arg(script_content);

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
