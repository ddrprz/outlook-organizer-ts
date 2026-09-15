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
}
