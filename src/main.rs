mod app;
mod backend;
mod report;
mod ui;

use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Frame, Terminal,
};
use tokio::sync::mpsc::{self, UnboundedReceiver};

use app::{AppState, ExplorerItemType, RoutingGranularity, RoutingModal, TransferMode, WizardStep};
use backend::{messages::BackendMessage, runner::BackendRunner};
use report::{html::generate_html_report, json::generate_audit_json};
use ui::{
    footer::render_footer,
    header::render_header,
    screens::{
        completion, deduplication, execution, explorer, filters, folders_mode, mailbox, pst_source,
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
    let mut worker_abort_file: Option<PathBuf> = None;

    // Disparar detección inicial de buzones MAPI de Outlook en segundo plano
    state.is_loading_mailboxes = true;
    BackendRunner::trigger_mailbox_discovery(
        if state.use_default_profile { None } else { Some(state.custom_profile_name.clone()) },
        tx.clone(),
    );

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

                    // Auditoría JSON automática (solo si es .exe de producción)
                    if let Ok(Some(path)) = generate_audit_json(&state, &status) {
                        state.log_event(format!("[AUDITORÍA] JSON guardado en: {}", path.display()));
                    }

                    state.step = WizardStep::Completion;
                }
                BackendMessage::MailboxesLoaded { items } => {
                    state.is_loading_mailboxes = false;
                    if !items.is_empty() {
                        state.discovered_mailboxes = items;
                        state.selected_mailbox_idx = 0;
                    }
                }
            }
        }

        terminal.draw(|f| draw_ui(f, &state))?;

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()? {
                // FILTRAR EVENTOS: Ignorar Release para prevenir saltos dobles en Windows
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Captura universal de Ctrl+C para parada segura o salida
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    if state.step == WizardStep::Execution {
                        state.progress.graceful_cancelling = true;
                        state.log_event("[SISTEMA] Ctrl+C capturado. Desmontando PST de forma segura con RemoveStore...".to_string());
                        if let Some(ref path) = worker_abort_file {
                            let _ = std::fs::File::create(path);
                        }
                        if let Some(ref mut child) = worker_child
                            && let Some(ref mut stdin) = child.stdin {
                            use tokio::io::AsyncWriteExt;
                            let _ = stdin.write_all(b"abort\n").await;
                        }
                    } else {
                        state.should_quit = true;
                    }
                    continue;
                }

                // Interacciones por pantalla
                match state.step {
                    WizardStep::Welcome => {
                        if state.is_editing_profile {
                            match key.code {
                                KeyCode::Esc => {
                                    state.is_editing_profile = false;
                                }
                                KeyCode::Enter => {
                                    state.is_editing_profile = false;
                                }
                                KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                                    state.use_default_profile = !state.use_default_profile;
                                }
                                KeyCode::Backspace => {
                                    if !state.use_default_profile {
                                        state.custom_profile_name.pop();
                                    }
                                }
                                KeyCode::Char(c) => {
                                    if state.use_default_profile {
                                        if c == '1' {
                                            state.use_default_profile = true;
                                        } else if c == '2' {
                                            state.use_default_profile = false;
                                        } else {
                                            state.use_default_profile = false;
                                            state.custom_profile_name.push(c);
                                        }
                                    } else {
                                        if state.custom_profile_name.len() < 45 {
                                            state.custom_profile_name.push(c);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            match key.code {
                                KeyCode::Up | KeyCode::Char('k') => {
                                    if state.welcome_menu_idx > 0 {
                                        state.welcome_menu_idx -= 1;
                                    }
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    if state.welcome_menu_idx + 1 < 4 {
                                        state.welcome_menu_idx += 1;
                                    }
                                }
                                KeyCode::Char('1') => {
                                    state.welcome_menu_idx = 0;
                                    start_import_or_explore(&mut state);
                                }
                                KeyCode::Char('2') => {
                                    state.welcome_menu_idx = 1;
                                    open_file_explorer(&mut state);
                                }
                                KeyCode::Char('3') | KeyCode::Char('p') | KeyCode::Char('P') => {
                                    state.welcome_menu_idx = 2;
                                    state.is_editing_profile = true;
                                }
                                KeyCode::Char('4') => {
                                    state.should_quit = true;
                                }
                                KeyCode::Enter => match state.welcome_menu_idx {
                                    0 => start_import_or_explore(&mut state),
                                    1 => open_file_explorer(&mut state),
                                    2 => state.is_editing_profile = true,
                                    3 => state.should_quit = true,
                                    _ => {}
                                },
                                KeyCode::Char('q') | KeyCode::Char('Q') => {
                                    state.should_quit = true;
                                }
                                _ => {}
                            }
                        }
                    },
                    WizardStep::FileExplorer => match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            if state.explorer.selected_idx > 0 {
                                state.explorer.selected_idx -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if state.explorer.selected_idx + 1 < state.explorer.entries.len() {
                                state.explorer.selected_idx += 1;
                            }
                        }
                        KeyCode::Enter => {
                            state.explorer.navigate_into_selected();
                        }
                        KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => {
                            state.explorer.navigate_up();
                        }
                        KeyCode::Char(' ') => {
                            if let Some(entry) = state.explorer.entries.get_mut(state.explorer.selected_idx)
                                && entry.item_type == ExplorerItemType::PstFile {
                                entry.selected = !entry.selected;
                            }
                        }
                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            let psts = state.explorer.collect_selected_psts();
                            if !psts.is_empty() {
                                state.discovered_psts = psts;
                                state.pst_scan_path = state.explorer.current_path.to_string_lossy().to_string();
                                state.selected_pst_table_idx = 0;
                                state.step = WizardStep::PstSource;
                            } else if !state.explorer.is_drives_view && state.explorer.current_path.exists() {
                                state.discovered_psts = Vec::new();
                                state.pst_scan_path = state.explorer.current_path.to_string_lossy().to_string();
                                state.selected_pst_table_idx = 0;
                                state.step = WizardStep::PstSource;
                            }
                        }
                        KeyCode::Char('d') | KeyCode::Char('D') => {
                            state.explorer.is_drives_view = true;
                            state.explorer.current_path = PathBuf::new();
                            state.explorer.refresh();
                        }
                        KeyCode::Esc => {
                            state.step = WizardStep::Welcome;
                        }
                        _ => {}
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
                        KeyCode::Char('e') | KeyCode::Char('E') => {
                            open_file_explorer(&mut state);
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
                        KeyCode::Up | KeyCode::Char('k') => {
                            if state.selected_mailbox_idx > 0 {
                                state.selected_mailbox_idx -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if state.selected_mailbox_idx + 1 < state.discovered_mailboxes.len() {
                                state.selected_mailbox_idx += 1;
                            }
                        }
                        KeyCode::Char(' ') => {
                            if let Some(item) = state.discovered_mailboxes.get_mut(state.selected_mailbox_idx) {
                                item.selected = !item.selected;
                            }
                            state.mailbox_warning_notice = None;
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            for item in &mut state.discovered_mailboxes {
                                item.selected = true;
                            }
                            state.mailbox_warning_notice = None;
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            for item in &mut state.discovered_mailboxes {
                                item.selected = false;
                            }
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            state.is_loading_mailboxes = true;
                            state.mailbox_warning_notice = None;
                            BackendRunner::trigger_mailbox_discovery(
                                if state.use_default_profile { None } else { Some(state.custom_profile_name.clone()) },
                                tx.clone(),
                            );
                        }
                        KeyCode::Enter => {
                            if state.selected_mailboxes().is_empty() {
                                state.mailbox_warning_notice = Some("Debe seleccionar al menos un buzón de destino para continuar.".to_string());
                            } else {
                                state.mailbox_warning_notice = None;
                                state.next_step();
                            }
                        }
                        KeyCode::Esc | KeyCode::Backspace => {
                            state.mailbox_warning_notice = None;
                            state.prev_step();
                        }
                        KeyCode::Char('q') | KeyCode::Char('Q') => state.should_quit = true,
                        _ => {}
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
                    WizardStep::Routing => {
                        match state.active_routing_modal {
                            RoutingModal::Criterion => match key.code {
                                KeyCode::Up | KeyCode::Char('k') => {
                                    if state.routing_modal_criterion_idx > 0 {
                                        state.routing_modal_criterion_idx -= 1;
                                    } else {
                                        state.routing_modal_criterion_idx = 2;
                                    }
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    if state.routing_modal_criterion_idx < 2 {
                                        state.routing_modal_criterion_idx += 1;
                                    } else {
                                        state.routing_modal_criterion_idx = 0;
                                    }
                                }
                                KeyCode::Enter => {
                                    match state.routing_modal_criterion_idx {
                                        0 => state.routing_granularity = RoutingGranularity::Mirror,
                                        1 => state.routing_granularity = RoutingGranularity::Years,
                                        _ => state.routing_granularity = RoutingGranularity::YearsAndMonths,
                                    }
                                    state.routing_modal_year_scope_idx = 0;
                                    state.active_routing_modal = RoutingModal::YearScope;
                                }
                                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                                    state.active_routing_modal = RoutingModal::None;
                                }
                                _ => {}
                            },
                            RoutingModal::YearScope => match key.code {
                                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                                    state.routing_modal_year_scope_idx = if state.routing_modal_year_scope_idx == 0 { 1 } else { 0 };
                                }
                                KeyCode::Enter => {
                                    if state.routing_modal_year_scope_idx == 0 {
                                        // Todos los años (por defecto)
                                        state.specific_year = None;
                                        state.routing_all_years = true;
                                        if state.routing_granularity == RoutingGranularity::Years {
                                            state.specific_month = None;
                                            state.routing_all_months = true;
                                            state.active_routing_modal = RoutingModal::None;
                                        } else {
                                            state.routing_modal_month_scope_idx = 0;
                                            state.active_routing_modal = RoutingModal::MonthScope;
                                        }
                                    } else {
                                        // Año específico
                                        state.active_routing_modal = RoutingModal::SpecificYear;
                                    }
                                }
                                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                                    state.active_routing_modal = RoutingModal::Criterion;
                                }
                                _ => {}
                            },
                            RoutingModal::SpecificYear => match key.code {
                                KeyCode::Char(c) if c.is_ascii_digit() => {
                                    if state.routing_input_year.len() < 4 {
                                        state.routing_input_year.push(c);
                                    }
                                }
                                KeyCode::Backspace => {
                                    state.routing_input_year.pop();
                                }
                                KeyCode::Enter => {
                                    if let Ok(y) = state.routing_input_year.parse::<u32>()
                                        && (1990..=2050).contains(&y) {
                                        state.specific_year = Some(y);
                                        state.routing_all_years = false;
                                        if state.routing_granularity == RoutingGranularity::Years {
                                            state.specific_month = None;
                                            state.routing_all_months = true;
                                            state.active_routing_modal = RoutingModal::None;
                                        } else {
                                            state.routing_modal_month_scope_idx = 0;
                                            state.active_routing_modal = RoutingModal::MonthScope;
                                        }
                                    }
                                }
                                KeyCode::Esc => {
                                    state.active_routing_modal = RoutingModal::YearScope;
                                }
                                _ => {}
                            },
                            RoutingModal::MonthScope => match key.code {
                                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                                    state.routing_modal_month_scope_idx = if state.routing_modal_month_scope_idx == 0 { 1 } else { 0 };
                                }
                                KeyCode::Enter => {
                                    if state.routing_modal_month_scope_idx == 0 {
                                        // Todos los meses:
                                        // Si specific_year es Some(y), abarca todos los meses de ese año específico.
                                        // Si specific_year es None, abarca todos los meses de todos los años.
                                        state.specific_month = None;
                                        state.routing_all_months = true;
                                        state.active_routing_modal = RoutingModal::None;
                                    } else {
                                        // Mes específico
                                        state.active_routing_modal = RoutingModal::SpecificMonth;
                                    }
                                }
                                KeyCode::Esc => {
                                    if state.specific_year.is_some() {
                                        state.active_routing_modal = RoutingModal::SpecificYear;
                                    } else {
                                        state.active_routing_modal = RoutingModal::YearScope;
                                    }
                                }
                                _ => {}
                            },
                            RoutingModal::SpecificMonth => match key.code {
                                KeyCode::Left | KeyCode::Char('h') => {
                                    if state.routing_input_month > 1 {
                                        state.routing_input_month -= 1;
                                    } else {
                                        state.routing_input_month = 12;
                                    }
                                }
                                KeyCode::Right | KeyCode::Char('l') => {
                                    if state.routing_input_month < 12 {
                                        state.routing_input_month += 1;
                                    } else {
                                        state.routing_input_month = 1;
                                    }
                                }
                                KeyCode::Enter => {
                                    state.specific_month = Some(state.routing_input_month);
                                    state.routing_all_months = false;
                                    state.active_routing_modal = RoutingModal::None;
                                }
                                KeyCode::Esc => {
                                    state.active_routing_modal = RoutingModal::MonthScope;
                                }
                                _ => {}
                            },
                            RoutingModal::None => match key.code {
                                KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Char('m') | KeyCode::Char('M') | KeyCode::Char(' ') => {
                                    state.active_routing_modal = RoutingModal::Criterion;
                                    state.routing_modal_criterion_idx = match state.routing_granularity {
                                        RoutingGranularity::Mirror => 0,
                                        RoutingGranularity::Years => 1,
                                        RoutingGranularity::YearsAndMonths => 2,
                                    };
                                }
                                _ => handle_navigation_keys(&mut state, key.code),
                            },
                        }
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
                            let selected_pst_paths: Vec<String> = state
                                .discovered_psts
                                .iter()
                                .filter(|p| p.selected)
                                .map(|p| p.path.clone())
                                .collect();

                            let total_psts = selected_pst_paths.len();
                            let first_pst_name = selected_pst_paths
                                .first()
                                .and_then(|p| std::path::Path::new(p).file_name())
                                .and_then(|n| n.to_str())
                                .unwrap_or("PST")
                                .to_string();

                            state.progress.total_psts = total_psts;
                            state.progress.current_pst_idx = if total_psts > 0 { 1 } else { 0 };
                            state.progress.current_pst_name = first_pst_name;
                            state.progress.current_pst_items = 0;
                            state.progress.current_pst_total = 0;
                            state.progress.global_items_processed = 0;
                            state.progress.global_items_total = 0;
                            state.progress.imported_count = 0;
                            state.progress.duplicates_skipped = 0;
                            state.progress.error_count = 0;
                            state.progress.graceful_cancelling = false;

                            state.next_step(); // Pasa a WizardStep::Execution
                            state.log_event("[SISTEMA] Iniciando subproceso PowerShell MAPI...".to_string());

                            let config = backend::runner::WorkerConfig {
                                profile_name: if state.use_default_profile { None } else { Some(state.custom_profile_name.clone()) },
                                psts: selected_pst_paths,
                                target_mailboxes: state.selected_mailboxes().iter().map(|m| m.display_name.clone()).collect(),
                                transfer_mode: match state.transfer_mode {
                                    TransferMode::Copy => "Copy".to_string(),
                                    TransferMode::Move => "Move".to_string(),
                                },
                                include_inbox: state.include_inbox,
                                include_sent: state.include_sent,
                                include_deleted: state.include_deleted,
                                include_custom_folders: state.include_custom_folders,
                                routing_enabled: state.routing_enabled,
                                routing_granularity: match state.routing_granularity {
                                    RoutingGranularity::Mirror => "Mirror".to_string(),
                                    RoutingGranularity::Years => "Years".to_string(),
                                    RoutingGranularity::YearsAndMonths => "YearsAndMonths".to_string(),
                                },
                                specific_year: state.specific_year,
                                specific_month: state.specific_month,
                                deduplication_enabled: state.deduplication_enabled,
                                deep_scan_enabled: state.deep_scan_enabled,
                                adaptive_throttling: state.adaptive_throttling_enabled,
                            };

                            match BackendRunner::spawn_worker(&config, tx.clone()) {
                                Ok((child, abort_path)) => {
                                    worker_child = Some(child);
                                    worker_abort_file = Some(abort_path);
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
                            if let Some(ref path) = worker_abort_file {
                                let _ = std::fs::File::create(path);
                            }
                            if let Some(ref mut child) = worker_child
                                && let Some(ref mut stdin) = child.stdin {
                                use tokio::io::AsyncWriteExt;
                                let _ = stdin.write_all(b"abort\n").await;
                            }
                        }
                        _ => {}
                    },
                    WizardStep::Completion => match key.code {
                        KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('Q') => {
                            state.should_quit = true;
                        }
                        KeyCode::Char('h') | KeyCode::Char('H') => {
                            match generate_html_report(&state, None) {
                                Ok(path) => {
                                    state.log_event(format!("[INFORME HTML] Generado exitosamente en: {}", path.display()));
                                }
                                Err(e) => {
                                    state.log_event(format!("[ERROR] Fallo al generar HTML: {}", e));
                                }
                            }
                        }
                        _ => {}
                    },
                }
            }
        }

    // Restauración limpia de la terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn start_import_or_explore(state: &mut AppState) {
    let default_path = Path::new(r"C:\Correo");
    if default_path.exists() {
        state.discovered_psts = crate::app::scan_folder_for_psts(default_path);
        state.pst_scan_path = r"C:\Correo".to_string();
        state.selected_pst_table_idx = 0;
        state.step = WizardStep::PstSource;
    } else {
        state.explorer.warning_notice = Some(
            "La ruta predeterminada 'C:\\Correo' no existe. Selecciona una carpeta o PST en el explorador.".to_string()
        );
        state.explorer.is_drives_view = true;
        state.explorer.current_path = PathBuf::new();
        state.explorer.refresh();
        state.step = WizardStep::FileExplorer;
    }
}

fn open_file_explorer(state: &mut AppState) {
    state.explorer.warning_notice = None;
    if !state.explorer.current_path.exists() {
        state.explorer.is_drives_view = true;
        state.explorer.current_path = PathBuf::new();
    }
    state.explorer.refresh();
    state.step = WizardStep::FileExplorer;
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
    render_header(f, chunks[0], state.step.title(), state.step.index(), 7);

    // 2. Body según el paso activo
    match state.step {
        WizardStep::Welcome => welcome::render(f, chunks[1], state),
        WizardStep::FileExplorer => explorer::render(f, chunks[1], state),
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
        WizardStep::Welcome => vec![("↑/↓", "Navegar"), ("Enter", "Seleccionar"), ("1-4", "Acceso"), ("P", "Perfil"), ("Q", "Salir")],
        WizardStep::FileExplorer => vec![("↑/↓", "Navegar"), ("Enter", "Abrir"), ("Backspace", "Subir"), ("Espacio", "Marcar"), ("C", "Confirmar"), ("Esc", "Volver")],
        WizardStep::PstSource => vec![("↑/↓", "Navegar"), ("Espacio", "Marcar"), ("E", "Explorar"), ("A/N", "Todos/Ninguno"), ("Enter", "Siguiente")],
        WizardStep::Mailbox => vec![("↑/↓", "Navegar"), ("Espacio", "Marcar"), ("A/N", "Todos/Ninguno"), ("R", "Recargar"), ("Enter", "Siguiente"), ("Esc", "Atrás")],
        WizardStep::FoldersMode => vec![("1-4", "Carpetas"), ("M", "Copiar/Mover"), ("Enter", "Siguiente"), ("Esc", "Atrás")],
        WizardStep::Routing => if state.active_routing_modal != RoutingModal::None {
            vec![("↑/↓", "Mover"), ("Enter", "Confirmar"), ("Esc/Q", "Cancelar")]
        } else {
            vec![("C", "Cambiar Criterio"), ("Enter", "Siguiente"), ("Esc", "Atrás")]
        },
        WizardStep::Deduplication => vec![("D", "Duplicados On/Off"), ("P", "Revisión Profunda"), ("Enter", "Siguiente")],
        WizardStep::Filters => vec![("T", "Throttling"), ("Enter", "Siguiente"), ("Esc", "Atrás")],
        WizardStep::Summary => vec![("Enter", "Iniciar Operación"), ("Esc", "Atrás"), ("q", "Salir")],
        WizardStep::Execution => vec![("Esc", "Parada Segura")],
        WizardStep::Completion => vec![("H", "Informe HTML"), ("Enter/q", "Salir")],
    };

    render_footer(f, chunks[2], &shortcuts);
}
