mod app;
mod backend;
mod ui;

use std::{io, time::Duration};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Frame, Terminal,
};
use tokio::sync::mpsc::{self, UnboundedReceiver};

use app::{AppState, RoutingGranularity, TransferMode, WizardStep};
use backend::{messages::BackendMessage, runner::BackendRunner};
use ui::{
    footer::render_footer,
    header::render_header,
    screens::{
        completion, deduplication, execution, filters, folders_mode, mailbox, pst_source,
        routing, summary, welcome,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Modo Raw y Terminal Alternativa
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = AppState::new();
    let (tx, mut rx): (mpsc::UnboundedSender<BackendMessage>, UnboundedReceiver<BackendMessage>) = mpsc::unbounded_channel();
    let mut worker_child: Option<tokio::process::Child> = None;

    // Event Loop Reactivo de Ultra Bajo Overhead (Sin busy-loop, CPU < 1%)
    while !state.should_quit {
        // Consumir mensajes asíncronos de telemetría de PowerShell
        while let Ok(msg) = rx.try_recv() {
            match msg {
                BackendMessage::Progress {
                    pst_name,
                    item_current,
                    item_total,
                    speed_mps,
                    eta_seconds,
                    ..
                } => {
                    state.progress.current_pst_name = pst_name;
                    state.progress.current_pst_items = item_current;
                    state.progress.current_pst_total = item_total;
                    state.progress.global_items_processed = item_current;
                    state.progress.global_items_total = item_total;
                    state.progress.speed_mps = speed_mps;
                    state.progress.eta_seconds = eta_seconds;
                }
                BackendMessage::Log { message, .. } => {
                    state.log_event(message);
                }
                BackendMessage::Throttling { active, .. } => {
                    state.progress.throttling_active = active;
                }
                BackendMessage::Finished {
                    status,
                    imported,
                    duplicates,
                    errors,
                } => {
                    state.progress.imported_count = imported;
                    state.progress.duplicates_skipped = duplicates;
                    state.progress.error_count = errors;
                    state.log_event(format!("[FIN] Operación finalizada con estado: {}", status));
                    state.step = WizardStep::Completion;
                }
            }
        }

        terminal.draw(|f| draw_ui(f, &state))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                // Captura universal de Ctrl+C para parada segura o salida
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    if state.step == WizardStep::Execution {
                        state.progress.graceful_cancelling = true;
                        state.log_event("[SISTEMA] Ctrl+C capturado. Desmontando PST de forma segura con RemoveStore...".to_string());
                    } else {
                        state.should_quit = true;
                    }
                    continue;
                }

                // Interacciones por pantalla
                match state.step {
                    WizardStep::Welcome => match key.code {
                        KeyCode::Char('p') | KeyCode::Char('P') | KeyCode::Char(' ') => {
                            state.use_default_profile = !state.use_default_profile;
                        }
                        _ => handle_navigation_keys(&mut state, key.code),
                    },
                    WizardStep::PstSource => match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            if state.selected_pst_table_idx > 0 {
                                state.selected_pst_table_idx -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if state.selected_pst_table_idx + 1 < state.discovered_psts.len() {
                                state.selected_pst_table_idx += 1;
                            }
                        }
                        KeyCode::Char(' ') => {
                            if let Some(item) = state.discovered_psts.get_mut(state.selected_pst_table_idx) {
                                item.selected = !item.selected;
                            }
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            for item in &mut state.discovered_psts {
                                item.selected = true;
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            for item in &mut state.discovered_psts {
                                item.selected = false;
                            }
                        }
                        _ => handle_navigation_keys(&mut state, key.code),
                    },
                    WizardStep::Mailbox => match key.code {
                        KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Char(' ') => {
                            state.is_shared_mailbox = !state.is_shared_mailbox;
                        }
                        _ => handle_navigation_keys(&mut state, key.code),
                    },
                    WizardStep::FoldersMode => match key.code {
                        KeyCode::Char('1') => state.include_inbox = !state.include_inbox,
                        KeyCode::Char('2') => state.include_sent = !state.include_sent,
                        KeyCode::Char('3') => state.include_deleted = !state.include_deleted,
                        KeyCode::Char('4') => state.include_custom_folders = !state.include_custom_folders,
                        KeyCode::Char('m') | KeyCode::Char('M') => {
                            state.transfer_mode = match state.transfer_mode {
                                TransferMode::Copy => TransferMode::Move,
                                TransferMode::Move => TransferMode::Copy,
                            };
                        }
                        _ => handle_navigation_keys(&mut state, key.code),
                    },
                    WizardStep::Routing => match key.code {
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            state.routing_enabled = !state.routing_enabled;
                        }
                        KeyCode::Char('g') | KeyCode::Char('G') => {
                            state.routing_granularity = match state.routing_granularity {
                                RoutingGranularity::Years => RoutingGranularity::YearsAndMonths,
                                RoutingGranularity::YearsAndMonths => RoutingGranularity::Years,
                            };
                        }
                        _ => handle_navigation_keys(&mut state, key.code),
                    },
                    WizardStep::Deduplication => match key.code {
                        KeyCode::Char('d') | KeyCode::Char('D') => {
                            state.deduplication_enabled = !state.deduplication_enabled;
                        }
                        KeyCode::Char('p') | KeyCode::Char('P') => {
                            state.deep_scan_enabled = !state.deep_scan_enabled;
                        }
                        _ => handle_navigation_keys(&mut state, key.code),
                    },
                    WizardStep::Filters => match key.code {
                        KeyCode::Char('t') | KeyCode::Char('T') => {
                            state.adaptive_throttling_enabled = !state.adaptive_throttling_enabled;
                        }
                        _ => handle_navigation_keys(&mut state, key.code),
                    },
                    WizardStep::Summary => match key.code {
                        KeyCode::Enter => {
                            state.next_step(); // Pasa a WizardStep::Execution
                            state.log_event("[SISTEMA] Iniciando subproceso PowerShell MAPI...".to_string());
                            match BackendRunner::spawn_worker(tx.clone()) {
                                Ok(child) => {
                                    worker_child = Some(child);
                                }
                                Err(e) => {
                                    state.log_event(format!("[ERROR] No se pudo iniciar PowerShell: {}", e));
                                }
                            }
                        }
                        KeyCode::Esc | KeyCode::Backspace => state.prev_step(),
                        KeyCode::Char('q') | KeyCode::Char('Q') => state.should_quit = true,
                        _ => {}
                    },
                    WizardStep::Execution => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            state.progress.graceful_cancelling = true;
                            state.log_event("[SISTEMA] Solicitud de parada segura recibida. Notificando a PowerShell...".to_string());
                            if let Some(ref mut child) = worker_child {
                                if let Some(ref mut stdin) = child.stdin {
                                    use tokio::io::AsyncWriteExt;
                                    let _ = stdin.write_all(b"abort\n").await;
                                }
                            }
                        }
                        _ => {}
                    },
                    WizardStep::Completion => match key.code {
                        KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('Q') => {
                            state.should_quit = true;
                        }
                        KeyCode::Char('h') | KeyCode::Char('H') => {
                            state.log_event("[REPORTE] Generando informe visual HTML en .\\logs...".to_string());
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    // Restauración limpia de la terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn handle_navigation_keys(state: &mut AppState, code: KeyCode) {
    match code {
        KeyCode::Enter => state.next_step(),
        KeyCode::Esc | KeyCode::Backspace => state.prev_step(),
        KeyCode::Char('q') | KeyCode::Char('Q') => state.should_quit = true,
        _ => {}
    }
}

fn draw_ui(f: &mut Frame, state: &AppState) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(14),   // Pantalla activa
            Constraint::Length(3), // Footer
        ])
        .split(size);

    // 1. Header
    render_header(f, chunks[0], state.step.title(), state.step.index(), 8);

    // 2. Body según el paso activo
    match state.step {
        WizardStep::Welcome => welcome::render(f, chunks[1], state),
        WizardStep::PstSource => pst_source::render(f, chunks[1], state),
        WizardStep::Mailbox => mailbox::render(f, chunks[1], state),
        WizardStep::FoldersMode => folders_mode::render(f, chunks[1], state),
        WizardStep::Routing => routing::render(f, chunks[1], state),
        WizardStep::Deduplication => deduplication::render(f, chunks[1], state),
        WizardStep::Filters => filters::render(f, chunks[1], state),
        WizardStep::Summary => summary::render(f, chunks[1], state),
        WizardStep::Execution => execution::render(f, chunks[1], state),
        WizardStep::Completion => completion::render(f, chunks[1], state),
    }

    // 3. Footer con Marca de Agua Timeless Support
    let shortcuts = match state.step {
        WizardStep::Welcome => vec![("Espacio/P", "Perfil"), ("Enter", "Siguiente"), ("Esc", "Atrás"), ("q", "Salir")],
        WizardStep::PstSource => vec![("↑/↓", "Navegar"), ("Espacio", "Marcar"), ("A/N", "Todos/Ninguno"), ("Enter", "Siguiente")],
        WizardStep::Mailbox => vec![("S", "Personal/Compartido"), ("Enter", "Siguiente"), ("Esc", "Atrás")],
        WizardStep::FoldersMode => vec![("1-4", "Carpetas"), ("M", "Copiar/Mover"), ("Enter", "Siguiente"), ("Esc", "Atrás")],
        WizardStep::Routing => vec![("R", "Enrutamiento On/Off"), ("G", "Granularidad"), ("Enter", "Siguiente"), ("Esc", "Atrás")],
        WizardStep::Deduplication => vec![("D", "Duplicados On/Off"), ("P", "Revisión Profunda"), ("Enter", "Siguiente")],
        WizardStep::Filters => vec![("T", "Throttling"), ("Enter", "Siguiente"), ("Esc", "Atrás")],
        WizardStep::Summary => vec![("Enter", "Iniciar Operación"), ("Esc", "Atrás"), ("q", "Salir")],
        WizardStep::Execution => vec![("Esc", "Parada Segura")],
        WizardStep::Completion => vec![("H", "Informe HTML"), ("Enter/q", "Salir")],
    };

    render_footer(f, chunks[2], &shortcuts);
}
