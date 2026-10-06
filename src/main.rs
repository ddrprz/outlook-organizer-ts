mod app;
mod backend;
mod report;
mod ui;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
};
use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::sync::mpsc::{self, UnboundedReceiver};

use app::{AppState, ExplorerItemType, RoutingGranularity, RoutingModal, TransferMode, WizardStep};
use backend::{messages::BackendMessage, runner::BackendRunner};
use report::{html::generate_html_report, json::generate_audit_json};
use ui::{
    footer::render_footer,
    header::render_header,
    screens::{
        completion, deduplication, execution, explorer, filters, folders_mode, mailbox,
        pst_detail_view, pst_source, routing, split_completion, split_config, split_execution,
        split_filter, split_select, split_summary, summary, welcome,
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
    let (tx, mut rx): (
        mpsc::UnboundedSender<BackendMessage>,
        UnboundedReceiver<BackendMessage>,
    ) = mpsc::unbounded_channel();
    let mut worker_child: Option<tokio::process::Child> = None;
    let mut worker_abort_file: Option<PathBuf> = None;

    // Disparar detección inicial de buzones MAPI de Outlook en segundo plano
    state.is_loading_mailboxes = true;
    BackendRunner::trigger_mailbox_discovery(
        if state.use_default_profile {
            None
        } else {
            Some(state.custom_profile_name.clone())
        },
        tx.clone(),
    );

    // Event Loop Reactivo de Ultra Bajo Overhead (Sin busy-loop, CPU < 1%)
    while !state.should_quit {
        // Consumir mensajes asíncronos de telemetría de PowerShell
        while let Ok(msg) = rx.try_recv() {
            match msg {
                BackendMessage::Progress {
                    pst_index,
                    pst_total,
                    pst_name,
                    item_current,
                    item_total,
                    speed_mps,
                    eta_seconds,
                    imported,
                    duplicates,
                    errors,
                } => {
                    state.progress.current_pst_idx = pst_index;
                    state.progress.total_psts = pst_total;
                    state.progress.current_pst_name = pst_name;
                    state.progress.current_pst_items = item_current;
                    state.progress.current_pst_total = item_total;
                    state.progress.global_items_processed = item_current;
                    state.progress.global_items_total = item_total;
                    state.progress.speed_mps = speed_mps;
                    state.progress.eta_seconds = eta_seconds;
                    state.progress.imported_count = imported;
                    state.progress.duplicates_skipped = duplicates;
                    state.progress.error_count = errors;
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
                    state.load_processed_items_from_temp();
                    state.log_event(format!("[FIN] Operación finalizada con estado: {}", status));

                    // Auditoría JSON automática (solo si es .exe de producción)
                    if let Ok(Some(path)) = generate_audit_json(&state, &status) {
                        state.json_audit_path = Some(path.clone());
                        state
                            .log_event(format!("[AUDITORÍA] JSON guardado en: {}", path.display()));
                    }

                    state.step = WizardStep::Completion;
                    state.cleanup_pause_file();
                }
                BackendMessage::SplitFinished {
                    status,
                    total_items,
                    total_extracted,
                    generated_psts,
                    errors: _,
                } => {
                    let final_count = if total_items > 0 {
                        total_items
                    } else {
                        total_extracted.unwrap_or(0)
                    };
                    state.split.execution_status = status.clone();
                    state.split.total_extracted = final_count;
                    state.split.generated_psts = generated_psts;
                    state.log_event(format!(
                        "[FIN] Separación de PSTs finalizada con estado: {}. Correos transferidos: {}",
                        status, final_count
                    ));
                    state.step = WizardStep::SplitCompletion;
                    state.cleanup_pause_file();
                }
                BackendMessage::MailboxesLoaded { items } => {
                    state.is_loading_mailboxes = false;
                    if !items.is_empty() {
                        state.discovered_mailboxes = items;
                        state.selected_mailbox_idx = 0;
                    }
                }
                BackendMessage::PstDetailLoaded { pst_path, pst_name, detail } => {
                    state.inspecting_psts.remove(&pst_path);
                    match detail {
                        Ok(d) => {
                            state.pst_details_cache.insert(pst_path.clone(), *d.clone());
                            if let app::PstDetailModalState::Loading { pst_path: ref p, .. } = state.pst_detail_modal
                                && p == &pst_path {
                                state.pst_folder_explorer.build_from_detail(&d);
                                state.pst_detail_modal = app::PstDetailModalState::Loaded(d.clone());
                            }
                            if state.split.source_psts.iter().any(|p| p.path == pst_path)
                                || state.split.source_pst.as_ref().map(|p| &p.path) == Some(&pst_path)
                            {
                                state.apply_pst_detail_to_split(&d);
                                if state.split.waiting_to_advance && state.step == WizardStep::SplitSelect {
                                    let all_cached = state.split.source_psts.iter().all(|p| state.pst_details_cache.contains_key(&p.path));
                                    if all_cached {
                                        state.split.waiting_to_advance = false;
                                        state.next_step();
                                    }
                                }
                            }
                            state.sync_folder_tree_from_selected_psts();
                        }
                        Err(err) => {
                            if let app::PstDetailModalState::Loading { pst_path: ref p, .. } = state.pst_detail_modal
                                && p == &pst_path {
                                state.pst_detail_modal = app::PstDetailModalState::Error {
                                    pst_name,
                                    message: err,
                                };
                            }
                            if let Some(ref src) = state.split.source_pst
                                && src.path == pst_path
                            {
                                state.split.is_scanning = false;
                                state.split.waiting_to_advance = false;
                            }
                        }
                    }
                }
                BackendMessage::PstInspectionProgress { pst_path, folder_name, scanned_items } => {
                    if let app::PstDetailModalState::Loading { pst_path: ref p, ref mut current_folder, scanned_items: ref mut items, .. } = state.pst_detail_modal
                        && p == &pst_path {
                        *current_folder = Some(folder_name.clone());
                        *items = scanned_items;
                    }
                    if let Some(ref src) = state.split.source_pst
                        && src.path == pst_path
                    {
                        state.split.is_scanning = true;
                        state.split.scanning_folder = Some(folder_name);
                        state.split.scanned_items = scanned_items;
                    }
                }
            }
        }

        // Disparar inspección en segundo plano para cualquier PST seleccionado que no tenga detalles cargados
        for pst in state.selected_uninspected_psts() {
            state.inspecting_psts.insert(pst.path.clone());
            let profile = if state.use_default_profile {
                None
            } else {
                Some(state.custom_profile_name.clone())
            };
            BackendRunner::trigger_pst_inspection(pst.path, pst.name, profile, tx.clone());
        }

        // Disparar inspección para los archivos PST marcados/seleccionados para separar si aún no están analizados
        if state.step == WizardStep::SplitSelect || state.step == WizardStep::SplitFilter {
            let psts_to_inspect: Vec<crate::app::PstItem> = if !state.split.source_psts.is_empty() {
                state.split.source_psts.clone()
            } else if let Some(ref src) = state.split.source_pst {
                vec![src.clone()]
            } else if let Some(pst) = state.discovered_psts.get(state.selected_pst_table_idx) {
                vec![pst.clone()]
            } else {
                Vec::new()
            };

            for pst in psts_to_inspect {
                if !state.pst_details_cache.contains_key(&pst.path)
                    && !state.inspecting_psts.contains(&pst.path)
                {
                    state.inspecting_psts.insert(pst.path.clone());
                    let profile = if state.use_default_profile {
                        None
                    } else {
                        Some(state.custom_profile_name.clone())
                    };
                    BackendRunner::trigger_pst_inspection(pst.path, pst.name, profile, tx.clone());
                }
            }
        }

        // Garantizar que si el detalle está cargado, el árbol de carpetas esté construido
        if let app::PstDetailModalState::Loaded(ref detail) = state.pst_detail_modal
            && state.pst_folder_explorer.nodes.is_empty() && !detail.folders.is_empty()
        {
            state.pst_folder_explorer.build_from_detail(detail);
        }

        terminal.draw(|f| draw_ui(f, &state))?;

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
        {
            // FILTRAR EVENTOS: Ignorar Release para prevenir saltos dobles en Windows
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Captura universal de Ctrl+C para parada segura o salida
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                if state.step == WizardStep::Execution {
                    if !state.progress.graceful_cancelling {
                        state.open_cancel_modal();
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
                            KeyCode::Esc | KeyCode::Enter => {
                                state.is_editing_profile = false;
                                state.is_loading_mailboxes = true;
                                BackendRunner::trigger_mailbox_discovery(
                                    if state.use_default_profile {
                                        None
                                    } else {
                                        Some(state.custom_profile_name.clone())
                                    },
                                    tx.clone(),
                                );
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
                                if state.welcome_menu_idx + 1 < 5 {
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
                            KeyCode::Char('3') => {
                                state.welcome_menu_idx = 2;
                                start_split_pst(&mut state);
                            }
                            KeyCode::Char('4') | KeyCode::Char('p') | KeyCode::Char('P') => {
                                state.welcome_menu_idx = 3;
                                state.is_editing_profile = true;
                            }
                            KeyCode::Char('5') => {
                                state.should_quit = true;
                            }
                            KeyCode::Enter => match state.welcome_menu_idx {
                                0 => start_import_or_explore(&mut state),
                                1 => open_file_explorer(&mut state),
                                2 => start_split_pst(&mut state),
                                3 => state.is_editing_profile = true,
                                4 => state.should_quit = true,
                                _ => {}
                            },
                            KeyCode::Char('q') | KeyCode::Char('Q') => {
                                state.should_quit = true;
                            }
                            _ => {}
                        }
                    }
                }
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
                        if let Some(entry) =
                            state.explorer.entries.get_mut(state.explorer.selected_idx)
                            && entry.item_type == ExplorerItemType::PstFile
                        {
                            entry.selected = !entry.selected;
                        }
                    }
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        let psts = state.explorer.collect_selected_psts();
                        if !psts.is_empty() {
                            state.discovered_psts = psts;
                            state.pst_scan_path =
                                state.explorer.current_path.to_string_lossy().to_string();
                            state.selected_pst_table_idx = 0;
                            state.pst_warning_notice = None;
                            state.step = state.explorer.return_step;
                            if state.step == WizardStep::SplitSelect {
                                state.init_split_from_selected_pst();
                            }
                        } else if !state.explorer.is_drives_view
                            && state.explorer.current_path.exists()
                        {
                            state.discovered_psts = Vec::new();
                            state.pst_scan_path =
                                state.explorer.current_path.to_string_lossy().to_string();
                            state.selected_pst_table_idx = 0;
                            state.pst_warning_notice = None;
                            state.step = state.explorer.return_step;
                            if state.step == WizardStep::SplitSelect {
                                state.init_split_from_selected_pst();
                            }
                        }
                    }
                    KeyCode::Char('b') | KeyCode::Char('B') => {
                        state.explorer.is_drives_view = true;
                        state.explorer.current_path = PathBuf::new();
                        state.explorer.refresh();
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        if let Some(entry) = state.explorer.entries.get(state.explorer.selected_idx)
                            && entry.item_type == ExplorerItemType::PstFile
                        {
                            let path = entry.path.to_string_lossy().to_string();
                            let name = entry.name.clone();
                            if state.open_pst_detail(path.clone(), name.clone()) {
                                BackendRunner::trigger_pst_inspection(
                                    path,
                                    name,
                                    if state.use_default_profile { None } else { Some(state.custom_profile_name.clone()) },
                                    tx.clone(),
                                );
                            }
                        }
                    }
                    KeyCode::Esc => {
                        state.step = state.explorer.return_step;
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
                        if let Some(item) =
                            state.discovered_psts.get_mut(state.selected_pst_table_idx)
                        {
                            item.selected = !item.selected;
                        }
                        state.pst_warning_notice = None;
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        if let Some(item) = state.discovered_psts.get(state.selected_pst_table_idx) {
                            let path = item.path.clone();
                            let name = item.name.clone();
                            if state.open_pst_detail(path.clone(), name.clone()) {
                                BackendRunner::trigger_pst_inspection(
                                    path,
                                    name,
                                    if state.use_default_profile { None } else { Some(state.custom_profile_name.clone()) },
                                    tx.clone(),
                                );
                            }
                        }
                    }
                    KeyCode::Char('e') | KeyCode::Char('E') => {
                        state.pst_warning_notice = None;
                        open_file_explorer(&mut state);
                    }
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        for item in &mut state.discovered_psts {
                            item.selected = true;
                        }
                        state.pst_warning_notice = None;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        for item in &mut state.discovered_psts {
                            item.selected = false;
                        }
                    }
                    KeyCode::Enter => {
                        if state.selected_psts().is_empty() {
                            state.pst_warning_notice = Some(
                                "Debe seleccionar al menos un archivo PST con la barra espaciadora para continuar."
                                    .to_string(),
                            );
                        } else {
                            state.pst_warning_notice = None;
                            state.next_step();
                        }
                    }
                    KeyCode::Esc | KeyCode::Backspace => {
                        state.pst_warning_notice = None;
                        state.prev_step();
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') => state.should_quit = true,
                    _ => {}
                },
                WizardStep::PstDetailView => {
                    match &state.pst_detail_modal {
                        app::PstDetailModalState::Loaded(_) => {
                            match key.code {
                                KeyCode::Up | KeyCode::Char('k') => {
                                    state.pst_folder_explorer.move_up();
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    state.pst_folder_explorer.move_down();
                                }
                                KeyCode::Enter | KeyCode::Char('e') | KeyCode::Char('E') => {
                                    state.pst_folder_explorer.toggle_expand();
                                }
                                KeyCode::Right | KeyCode::Char('l') => {
                                    state.pst_folder_explorer.expand();
                                }
                                KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => {
                                    state.pst_folder_explorer.collapse();
                                }
                                KeyCode::Char(' ') => {
                                    state.pst_folder_explorer.toggle_select();
                                }
                                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Char('d') | KeyCode::Char('D') => {
                                    state.close_pst_detail();
                                }
                                _ => {}
                            }
                        }
                        _ => {
                            match key.code {
                                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Enter => {
                                    state.close_pst_detail();
                                }
                                _ => {}
                            }
                        }
                    }
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
                        if let Some(item) = state
                            .discovered_mailboxes
                            .get_mut(state.selected_mailbox_idx)
                        {
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
                            if state.use_default_profile {
                                None
                            } else {
                                Some(state.custom_profile_name.clone())
                            },
                            tx.clone(),
                        );
                    }
                    KeyCode::Enter => {
                        if state.selected_mailboxes().is_empty() {
                            state.mailbox_warning_notice = Some(
                                "Debe seleccionar al menos un buzón de destino para continuar."
                                    .to_string(),
                            );
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
                    KeyCode::Up | KeyCode::Char('k') => state.folder_tree.move_up(),
                    KeyCode::Down | KeyCode::Char('j') => state.folder_tree.move_down(),
                    KeyCode::Char('e')
                    | KeyCode::Char('E')
                    | KeyCode::Left
                    | KeyCode::Right => state.folder_tree.toggle_expand(),
                    KeyCode::Char(' ') => {
                        state.folder_tree.toggle_select();
                        state.sync_legacy_folder_flags();
                    }
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        state.folder_tree.select_all();
                        state.sync_legacy_folder_flags();
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        state.folder_tree.deselect_all();
                        state.sync_legacy_folder_flags();
                    }
                    KeyCode::Char('1') => {
                        state.include_inbox = !state.include_inbox;
                    }
                    KeyCode::Char('2') => {
                        state.include_sent = !state.include_sent;
                    }
                    KeyCode::Char('3') => {
                        state.include_deleted = !state.include_deleted;
                    }
                    KeyCode::Char('4') => {
                        state.include_custom_folders = !state.include_custom_folders;
                    }
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
                            KeyCode::Char('1') => {
                                state.routing_granularity = RoutingGranularity::Mirror;
                                state.routing_modal_criterion_idx = 0;
                                state.active_routing_modal = RoutingModal::None;
                            }
                            KeyCode::Char('2') => {
                                state.routing_granularity = RoutingGranularity::Years;
                                state.routing_modal_criterion_idx = 1;
                                state.active_routing_modal = RoutingModal::None;
                            }
                            KeyCode::Char('3') => {
                                state.routing_granularity = RoutingGranularity::YearsAndMonths;
                                state.routing_modal_criterion_idx = 2;
                                state.active_routing_modal = RoutingModal::None;
                            }
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
                                state.routing_granularity = match state.routing_modal_criterion_idx {
                                    0 => RoutingGranularity::Mirror,
                                    1 => RoutingGranularity::Years,
                                    _ => RoutingGranularity::YearsAndMonths,
                                };
                                state.active_routing_modal = RoutingModal::None;
                            }
                            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                                state.active_routing_modal = RoutingModal::None;
                            }
                            _ => {}
                        },
                        RoutingModal::YearScope => {
                            let available_years = state.available_years_from_psts();
                            let max_idx = available_years.len();
                            match key.code {
                                KeyCode::Up | KeyCode::Char('k') => {
                                    if state.routing_modal_year_cursor > 0 {
                                        state.routing_modal_year_cursor -= 1;
                                    } else {
                                        state.routing_modal_year_cursor = max_idx;
                                    }
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    if state.routing_modal_year_cursor < max_idx {
                                        state.routing_modal_year_cursor += 1;
                                    } else {
                                        state.routing_modal_year_cursor = 0;
                                    }
                                }
                                KeyCode::Char(' ') => {
                                    if state.routing_modal_year_cursor == 0 {
                                        state.set_all_years();
                                    } else if let Some(&y) = available_years.get(state.routing_modal_year_cursor - 1) {
                                        state.toggle_year(y);
                                    }
                                }
                                KeyCode::Char('t') | KeyCode::Char('T') => {
                                    state.set_all_years();
                                    state.routing_modal_year_cursor = 0;
                                }
                                KeyCode::Enter => {
                                    if state.routing_modal_year_cursor == 0 {
                                        state.set_all_years();
                                    } else if state.selected_years.is_empty()
                                        && let Some(&y) = available_years.get(state.routing_modal_year_cursor - 1) {
                                            state.toggle_year(y);
                                    }
                                    state.active_routing_modal = RoutingModal::None;
                                }
                                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                                    state.active_routing_modal = RoutingModal::None;
                                }
                                _ => {}
                            }
                        }
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
                                    && (1990..=2050).contains(&y)
                                {
                                    state.set_all_years();
                                    state.toggle_year(y);
                                    state.active_routing_modal = RoutingModal::None;
                                }
                            }
                            KeyCode::Esc => {
                                state.active_routing_modal = RoutingModal::YearScope;
                            }
                            _ => {}
                        },
                        RoutingModal::MonthScope => {
                            let available_months = state.available_months_from_psts();
                            let max_idx = 2 + available_months.len();
                            match key.code {
                                KeyCode::Up | KeyCode::Char('k') => {
                                    if state.routing_modal_month_cursor > 0 {
                                        state.routing_modal_month_cursor -= 1;
                                    } else {
                                        state.routing_modal_month_cursor = max_idx;
                                    }
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    if state.routing_modal_month_cursor < max_idx {
                                        state.routing_modal_month_cursor += 1;
                                    } else {
                                        state.routing_modal_month_cursor = 0;
                                    }
                                }
                                KeyCode::Char('1') => {
                                    state.set_first_half_months();
                                    state.routing_modal_month_cursor = 1;
                                }
                                KeyCode::Char('2') => {
                                    state.set_second_half_months();
                                    state.routing_modal_month_cursor = 2;
                                }
                                KeyCode::Char('t') | KeyCode::Char('T') => {
                                    state.set_all_months();
                                    state.routing_modal_month_cursor = 0;
                                }
                                KeyCode::Char(' ') => {
                                    match state.routing_modal_month_cursor {
                                        0 => state.set_all_months(),
                                        1 => state.set_first_half_months(),
                                        2 => state.set_second_half_months(),
                                        idx => {
                                            if let Some(&m) = available_months.get(idx - 3) {
                                                state.toggle_month(m);
                                            }
                                        }
                                    }
                                }
                                KeyCode::Enter => {
                                    if state.routing_modal_month_cursor == 0 {
                                        state.set_all_months();
                                    } else if state.routing_modal_month_cursor == 1 {
                                        state.set_first_half_months();
                                    } else if state.routing_modal_month_cursor == 2 {
                                        state.set_second_half_months();
                                    } else if state.selected_months.is_empty()
                                        && let Some(&m) = available_months.get(state.routing_modal_month_cursor - 3) {
                                            state.toggle_month(m);
                                    }
                                    state.active_routing_modal = RoutingModal::None;
                                }
                                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                                    state.active_routing_modal = RoutingModal::None;
                                }
                                _ => {}
                            }
                        }
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
                                state.set_all_months();
                                state.toggle_month(state.routing_input_month);
                                state.active_routing_modal = RoutingModal::None;
                            }
                            KeyCode::Esc => {
                                state.active_routing_modal = RoutingModal::MonthScope;
                            }
                            _ => {}
                        },
                        RoutingModal::None => match key.code {
                            KeyCode::Char('1') => {
                                state.routing_granularity = RoutingGranularity::Mirror;
                                state.routing_modal_criterion_idx = 0;
                            }
                            KeyCode::Char('2') => {
                                state.routing_granularity = RoutingGranularity::Years;
                                state.routing_modal_criterion_idx = 1;
                            }
                            KeyCode::Char('3') => {
                                state.routing_granularity = RoutingGranularity::YearsAndMonths;
                                state.routing_modal_criterion_idx = 2;
                            }
                            KeyCode::Left
                            | KeyCode::Up
                            | KeyCode::Char('h')
                            | KeyCode::Char('k') => {
                                state.routing_granularity = match state.routing_granularity {
                                    RoutingGranularity::Mirror => RoutingGranularity::YearsAndMonths,
                                    RoutingGranularity::Years => RoutingGranularity::Mirror,
                                    RoutingGranularity::YearsAndMonths => RoutingGranularity::Years,
                                };
                                state.routing_modal_criterion_idx = match state.routing_granularity {
                                    RoutingGranularity::Mirror => 0,
                                    RoutingGranularity::Years => 1,
                                    RoutingGranularity::YearsAndMonths => 2,
                                };
                            }
                            KeyCode::Right
                            | KeyCode::Down
                            | KeyCode::Tab
                            | KeyCode::Char('l')
                            | KeyCode::Char('j') => {
                                state.routing_granularity = match state.routing_granularity {
                                    RoutingGranularity::Mirror => RoutingGranularity::Years,
                                    RoutingGranularity::Years => RoutingGranularity::YearsAndMonths,
                                    RoutingGranularity::YearsAndMonths => RoutingGranularity::Mirror,
                                };
                                state.routing_modal_criterion_idx = match state.routing_granularity {
                                    RoutingGranularity::Mirror => 0,
                                    RoutingGranularity::Years => 1,
                                    RoutingGranularity::YearsAndMonths => 2,
                                };
                            }
                            KeyCode::Char('c') | KeyCode::Char('C') => {
                                state.routing_modal_criterion_idx = match state.routing_granularity {
                                    RoutingGranularity::Mirror => 0,
                                    RoutingGranularity::Years => 1,
                                    RoutingGranularity::YearsAndMonths => 2,
                                };
                                state.active_routing_modal = RoutingModal::Criterion;
                            }
                            KeyCode::Char('f') | KeyCode::Char('F') | KeyCode::Char('a') | KeyCode::Char('A') => {
                                state.routing_modal_year_cursor = 0;
                                state.active_routing_modal = RoutingModal::YearScope;
                            }
                            KeyCode::Char('m') | KeyCode::Char('M') => {
                                state.routing_modal_month_cursor = 0;
                                state.active_routing_modal = RoutingModal::MonthScope;
                            }
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                state.set_all_years();
                                state.set_all_months();
                            }
                            _ => handle_navigation_keys(&mut state, key.code),
                        },
                    }
                }
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
                        state.log_event(
                            "[SISTEMA] Iniciando subproceso PowerShell MAPI...".to_string(),
                        );

                        let config = backend::runner::WorkerConfig {
                            profile_name: if state.use_default_profile {
                                None
                            } else {
                                Some(state.custom_profile_name.clone())
                            },
                            psts: selected_pst_paths,
                            target_mailboxes: state
                                .selected_mailboxes()
                                .iter()
                                .map(|m| m.display_name.clone())
                                .collect(),
                            transfer_mode: match state.transfer_mode {
                                TransferMode::Copy => "Copy".to_string(),
                                TransferMode::Move => "Move".to_string(),
                            },
                            include_inbox: state.include_inbox,
                            include_sent: state.include_sent,
                            include_deleted: state.include_deleted,
                            include_custom_folders: state.include_custom_folders,
                            selected_folder_paths: state.folder_tree.selected_paths(),
                            routing_enabled: state.routing_enabled,
                            routing_granularity: match state.routing_granularity {
                                RoutingGranularity::Mirror => "Mirror".to_string(),
                                RoutingGranularity::Years => "Years".to_string(),
                                RoutingGranularity::YearsAndMonths => "YearsAndMonths".to_string(),
                            },
                            specific_year: state.specific_year,
                            specific_month: state.specific_month,
                            specific_years: if state.routing_all_years {
                                Vec::new()
                            } else {
                                state.selected_years.iter().copied().collect()
                            },
                            specific_months: if state.routing_all_months {
                                Vec::new()
                            } else {
                                state.selected_months.iter().copied().collect()
                            },
                            deduplication_enabled: state.deduplication_enabled,
                            deep_scan_enabled: state.deep_scan_enabled,
                            adaptive_throttling: state.adaptive_throttling_enabled,
                        };

                        match BackendRunner::spawn_worker(&config, tx.clone()) {
                            Ok((child, abort_path, pause_path)) => {
                                worker_child = Some(child);
                                worker_abort_file = Some(abort_path);
                                state.pause_file = Some(pause_path);
                            }
                            Err(e) => {
                                state.log_event(format!(
                                    "[ERROR] No se pudo iniciar PowerShell: {}",
                                    e
                                ));
                            }
                        }
                    }
                    KeyCode::Esc | KeyCode::Backspace => state.prev_step(),
                    KeyCode::Char('q') | KeyCode::Char('Q') => state.should_quit = true,
                    _ => {}
                },
                WizardStep::Execution => {
                    if state.show_cancel_modal {
                        match key.code {
                            KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => {
                                state.cancel_modal_selected_yes = !state.cancel_modal_selected_yes;
                            }
                            KeyCode::Char('s') | KeyCode::Char('S') => {
                                state.close_cancel_modal();
                                state.progress.graceful_cancelling = true;
                                state.log_event("[SISTEMA] Parada segura confirmada por el usuario. Desmontando PST con RemoveStore...".to_string());
                                if let Some(ref path) = worker_abort_file {
                                    let _ = std::fs::File::create(path);
                                }
                                if let Some(ref mut child) = worker_child
                                    && let Some(ref mut stdin) = child.stdin
                                {
                                    use tokio::io::AsyncWriteExt;
                                    let _ = stdin.write_all(b"abort\n").await;
                                }
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                state.close_cancel_modal();
                                state.log_event("[SISTEMA] Parada segura descartada. Continuando importación...".to_string());
                            }
                            KeyCode::Enter => {
                                if state.cancel_modal_selected_yes {
                                    state.close_cancel_modal();
                                    state.progress.graceful_cancelling = true;
                                    state.log_event("[SISTEMA] Parada segura confirmada por el usuario. Desmontando PST con RemoveStore...".to_string());
                                    if let Some(ref path) = worker_abort_file {
                                        let _ = std::fs::File::create(path);
                                    }
                                    if let Some(ref mut child) = worker_child
                                        && let Some(ref mut stdin) = child.stdin
                                    {
                                        use tokio::io::AsyncWriteExt;
                                        let _ = stdin.write_all(b"abort\n").await;
                                    }
                                } else {
                                    state.close_cancel_modal();
                                    state.log_event("[SISTEMA] Parada segura descartada. Continuando importación...".to_string());
                                }
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('p') | KeyCode::Char('P') => {
                                state.toggle_pause();
                            }
                            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q')
                                if !state.progress.graceful_cancelling =>
                            {
                                state.open_cancel_modal();
                            }
                            _ => {}
                        }
                    }
                }
                WizardStep::Completion => match key.code {
                    KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('Q') => {
                        state.should_quit = true;
                    }
                    KeyCode::Char('h') | KeyCode::Char('H') => {
                        match generate_html_report(&state, None) {
                            Ok(path) => {
                                state.html_report_path = Some(path.clone());
                                state.log_event(format!(
                                    "[INFORME HTML] Generado exitosamente en: {}",
                                    path.display()
                                ));
                                let _ = std::process::Command::new("cmd")
                                    .args(["/C", "start", "", &path.to_string_lossy()])
                                    .spawn();
                            }
                            Err(e) => {
                                state.log_event(format!("[ERROR] Fallo al generar HTML: {}", e));
                            }
                        }
                    }
                    KeyCode::Char('o') | KeyCode::Char('O') => {
                        if let Some(ref path) = state.html_report_path {
                            let _ = std::process::Command::new("cmd")
                                .args(["/C", "start", "", &path.to_string_lossy()])
                                .spawn();
                        }
                    }
                    _ => {}
                },
                WizardStep::SplitSelect => match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if state.selected_pst_table_idx > 0 {
                            state.selected_pst_table_idx -= 1;
                            state.init_split_from_selected_pst();
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if state.selected_pst_table_idx + 1 < state.discovered_psts.len() {
                            state.selected_pst_table_idx += 1;
                            state.init_split_from_selected_pst();
                        }
                    }
                    KeyCode::Char(' ') => {
                        if let Some(item) = state.discovered_psts.get_mut(state.selected_pst_table_idx) {
                            item.selected = !item.selected;
                        }
                        let marked: Vec<_> = state.discovered_psts.iter().filter(|p| p.selected).cloned().collect();
                        if !marked.is_empty() {
                            state.split.source_psts = marked.clone();
                            state.split.source_pst = marked.first().cloned();
                        } else if let Some(current) = state.discovered_psts.get(state.selected_pst_table_idx) {
                            state.split.source_psts = vec![current.clone()];
                            state.split.source_pst = Some(current.clone());
                        }
                        state.sync_split_available_filters();
                    }
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        for p in &mut state.discovered_psts {
                            p.selected = true;
                        }
                        let marked: Vec<_> = state.discovered_psts.iter().filter(|p| p.selected).cloned().collect();
                        state.split.source_psts = marked.clone();
                        state.split.source_pst = marked.first().cloned();
                        state.sync_split_available_filters();
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        for p in &mut state.discovered_psts {
                            p.selected = false;
                        }
                        if let Some(current) = state.discovered_psts.get(state.selected_pst_table_idx) {
                            state.split.source_psts = vec![current.clone()];
                            state.split.source_pst = Some(current.clone());
                        }
                        state.sync_split_available_filters();
                    }
                    KeyCode::Char('e') | KeyCode::Char('E') => {
                        state.explorer.return_step = WizardStep::SplitSelect;
                        open_file_explorer(&mut state);
                    }
                    KeyCode::Enter => {
                        if !state.discovered_psts.is_empty() {
                            let any_selected = state.discovered_psts.iter().any(|p| p.selected);
                            if !any_selected {
                                if let Some(item) = state.discovered_psts.get_mut(state.selected_pst_table_idx) {
                                    item.selected = true;
                                }
                                let marked: Vec<_> = state.discovered_psts.iter().filter(|p| p.selected).cloned().collect();
                                state.split.source_psts = marked.clone();
                                state.split.source_pst = marked.first().cloned();
                                state.sync_split_available_filters();
                            }

                            let all_cached = state.split.source_psts.iter().all(|p| state.pst_details_cache.contains_key(&p.path));
                            if all_cached && !state.split.source_psts.is_empty() {
                                state.next_step();
                            } else {
                                state.split.waiting_to_advance = true;
                            }
                        }
                    }
                    KeyCode::Esc | KeyCode::Backspace => {
                        if state.split.waiting_to_advance {
                            state.split.waiting_to_advance = false;
                        } else {
                            state.step = WizardStep::Welcome;
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        state.should_quit = true;
                    }
                    _ => {}
                },
                WizardStep::SplitFilter => match key.code {
                    KeyCode::Tab => {
                        state.split.config_cursor = if state.split.config_cursor == 0 { 1 } else { 0 };
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if state.split.config_cursor == 0 {
                            if state.split.year_cursor > 0 {
                                state.split.year_cursor -= 1;
                            }
                        } else if state.split.month_cursor > 0 {
                            state.split.month_cursor -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if state.split.config_cursor == 0 {
                            if state.split.year_cursor + 1 < state.split.available_years.len() {
                                state.split.year_cursor += 1;
                            }
                        } else if state.split.month_cursor + 1 < state.split.available_months.len() {
                            state.split.month_cursor += 1;
                        }
                    }
                    KeyCode::Char(' ') => {
                        if state.split.config_cursor == 0 {
                            if let Some(&y) = state.split.available_years.get(state.split.year_cursor) {
                                if state.split.selected_years.contains(&y) {
                                    state.split.selected_years.remove(&y);
                                } else {
                                    state.split.selected_years.insert(y);
                                }
                                state.update_split_available_months();
                            }
                        } else if let Some(&m) = state.split.available_months.get(state.split.month_cursor) {
                            if state.split.selected_months.contains(&m) {
                                state.split.selected_months.remove(&m);
                            } else {
                                state.split.selected_months.insert(m);
                            }
                        }
                    }
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        if state.split.config_cursor == 0 {
                            state.split.selected_years = state.split.available_years.iter().copied().collect();
                            state.update_split_available_months();
                        } else {
                            state.split.selected_months = state.split.available_months.iter().copied().collect();
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        if state.split.config_cursor == 0 {
                            state.split.selected_years.clear();
                            state.update_split_available_months();
                        } else {
                            state.split.selected_months.clear();
                        }
                    }
                    KeyCode::Char('1') => {
                        let h1: std::collections::BTreeSet<u32> = state.split.available_months.iter().copied().filter(|&m| m <= 6).collect();
                        state.split.selected_months = h1;
                    }
                    KeyCode::Char('2') => {
                        let h2: std::collections::BTreeSet<u32> = state.split.available_months.iter().copied().filter(|&m| m >= 7).collect();
                        state.split.selected_months = h2;
                    }
                    KeyCode::Char('t') | KeyCode::Char('T') => {
                        state.split.selected_months = state.split.available_months.iter().copied().collect();
                    }
                    KeyCode::Char('i') | KeyCode::Char('I') => {
                        state.split.include_inbox = !state.split.include_inbox;
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        state.split.include_sent = !state.split.include_sent;
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        state.split.include_deleted = !state.split.include_deleted;
                    }
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        state.split.include_custom_folders = !state.split.include_custom_folders;
                    }
                    KeyCode::Enter => {
                        state.next_step();
                    }
                    KeyCode::Esc | KeyCode::Backspace => {
                        state.prev_step();
                    }
                    _ => {}
                },
                WizardStep::SplitConfig => {
                    if state.split.is_editing_output_dir {
                        match key.code {
                            KeyCode::Enter | KeyCode::Esc => {
                                state.split.is_editing_output_dir = false;
                            }
                            KeyCode::Backspace => {
                                state.split.output_dir.pop();
                            }
                            KeyCode::Char(c) if state.split.output_dir.len() < 240 => {
                                state.split.output_dir.push(c);
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('1') => {
                                state.split.partition_mode = crate::app::SplitPartitionMode::ByYear;
                            }
                            KeyCode::Char('2') => {
                                state.split.partition_mode = crate::app::SplitPartitionMode::ByYearMonth;
                            }
                            KeyCode::Char('3') => {
                                state.split.partition_mode = crate::app::SplitPartitionMode::SinglePst;
                            }
                            KeyCode::Char('m') | KeyCode::Char('M') => {
                                state.split.transfer_mode = match state.split.transfer_mode {
                                    crate::app::SplitTransferMode::Copy => crate::app::SplitTransferMode::Move,
                                    crate::app::SplitTransferMode::Move => crate::app::SplitTransferMode::Copy,
                                };
                            }
                            KeyCode::Char('o') | KeyCode::Char('O') => {
                                state.split.is_editing_output_dir = true;
                            }
                            KeyCode::Enter => {
                                state.next_step();
                            }
                            KeyCode::Esc | KeyCode::Backspace => {
                                state.prev_step();
                            }
                            _ => {}
                        }
                    }
                }
                WizardStep::SplitSummary => match key.code {
                    KeyCode::Enter => {
                        state.progress.reset();
                        state.progress.current_pst_name = "Iniciando partición...".to_string();
                        state.log_event("[SPLIT] Iniciando partición de archivo PST...".to_string());

                        let source_pst_paths: Vec<String> = if !state.split.source_psts.is_empty() {
                            state.split.source_psts.iter().map(|p| p.path.clone()).collect()
                        } else if let Some(ref src) = state.split.source_pst {
                            vec![src.path.clone()]
                        } else {
                            Vec::new()
                        };

                        let config = crate::backend::runner::SplitWorkerConfig {
                            profile_name: if state.use_default_profile { None } else { Some(state.custom_profile_name.clone()) },
                            source_pst_path: source_pst_paths.first().cloned().unwrap_or_default(),
                            source_pst_paths,
                            output_dir: state.split.output_dir.clone(),
                            partition_mode: match state.split.partition_mode {
                                crate::app::SplitPartitionMode::ByYear => "ByYear".to_string(),
                                crate::app::SplitPartitionMode::ByYearMonth => "ByYearMonth".to_string(),
                                crate::app::SplitPartitionMode::SinglePst => "SinglePst".to_string(),
                            },
                            transfer_mode: match state.split.transfer_mode {
                                crate::app::SplitTransferMode::Copy => "Copy".to_string(),
                                crate::app::SplitTransferMode::Move => "Move".to_string(),
                            },
                            selected_years: state.split.selected_years.iter().copied().collect(),
                            selected_months: state.split.selected_months.iter().copied().collect(),
                            include_inbox: state.split.include_inbox,
                            include_sent: state.split.include_sent,
                            include_deleted: state.split.include_deleted,
                            include_custom_folders: state.split.include_custom_folders,
                            adaptive_throttling: state.adaptive_throttling_enabled,
                        };

                        match BackendRunner::spawn_split_worker(&config, tx.clone()) {
                            Ok((child, abort_path, pause_path)) => {
                                worker_child = Some(child);
                                worker_abort_file = Some(abort_path);
                                state.pause_file = Some(pause_path);
                                state.step = WizardStep::SplitExecution;
                            }
                            Err(e) => {
                                state.log_event(format!("[ERROR] No se pudo iniciar el proceso de separación: {}", e));
                            }
                        }
                    }
                    KeyCode::Esc | KeyCode::Backspace => {
                        state.prev_step();
                    }
                    _ => {}
                },
                WizardStep::SplitExecution => match key.code {
                    KeyCode::Char('p') | KeyCode::Char('P') => {
                        state.toggle_pause();
                    }
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q')
                        if !state.progress.graceful_cancelling =>
                    {
                        state.progress.graceful_cancelling = true;
                        if let Some(ref path) = worker_abort_file {
                            let _ = std::fs::File::create(path);
                        }
                        state.log_event("[ABORT] Señal de parada enviada al particionador de PSTs.".to_string());
                    }
                    _ => {}
                },
                WizardStep::SplitCompletion => match key.code {
                    KeyCode::Char('o') | KeyCode::Char('O') => {
                        let _ = std::process::Command::new("explorer")
                            .arg(&state.split.output_dir)
                            .spawn();
                    }
                    KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                        state.step = WizardStep::Welcome;
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
        state.pst_warning_notice = None;
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

fn start_split_pst(state: &mut AppState) {
    let default_path = Path::new(r"C:\Correo");
    if default_path.exists() && state.discovered_psts.is_empty() {
        state.discovered_psts = crate::app::scan_folder_for_psts(default_path);
        state.pst_scan_path = r"C:\Correo".to_string();
    }
    if state.discovered_psts.is_empty() {
        state.explorer.warning_notice = Some(
            "Selecciona la carpeta o archivo .pst que deseas particionar.".to_string(),
        );
        state.explorer.return_step = WizardStep::SplitSelect;
        open_file_explorer(state);
    } else {
        state.selected_pst_table_idx = 0;
        state.init_split_from_selected_pst();
        state.step = WizardStep::SplitSelect;
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
    render_header(f, chunks[0], state.step.title(), state.step.index(), state.step.total_steps());

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
        WizardStep::PstDetailView => pst_detail_view::render(f, chunks[1], state),
        WizardStep::SplitSelect => split_select::render(f, chunks[1], state),
        WizardStep::SplitFilter => split_filter::render(f, chunks[1], state),
        WizardStep::SplitConfig => split_config::render(f, chunks[1], state),
        WizardStep::SplitSummary => split_summary::render(f, chunks[1], state),
        WizardStep::SplitExecution => split_execution::render(f, chunks[1], state),
        WizardStep::SplitCompletion => split_completion::render(f, chunks[1], state),
    }

    // 3. Footer con Marca de Agua Timeless Support
    let shortcuts = match state.step {
        WizardStep::Welcome => vec![
            ("↑/↓", "Navegar"),
            ("Enter", "Seleccionar"),
            ("1-5", "Acceso"),
            ("P", "Perfil"),
            ("Q", "Salir"),
        ],
        WizardStep::FileExplorer => vec![
            ("↑/↓", "Navegar"),
            ("Enter", "Abrir"),
            ("Backspace", "Subir"),
            ("Espacio", "Marcar"),
            ("C", "Confirmar"),
            ("Esc", "Volver"),
        ],
        WizardStep::PstSource => vec![
            ("↑/↓", "Navegar"),
            ("Espacio", "Marcar"),
            ("E", "Explorar"),
            ("A/N", "Todos/Ninguno"),
            ("Enter", "Siguiente"),
        ],
        WizardStep::Mailbox => vec![
            ("↑/↓", "Navegar"),
            ("Espacio", "Marcar"),
            ("A/N", "Todos/Ninguno"),
            ("R", "Recargar"),
            ("Enter", "Siguiente"),
            ("Esc", "Atrás"),
        ],
        WizardStep::FoldersMode => vec![
            ("↑/↓", "Navegar"),
            ("E", "Desplegar"),
            ("Espacio", "Seleccionar"),
            ("A/N", "Todas/Ninguna"),
            ("M", "Copiar/Mover"),
            ("Enter", "Siguiente"),
            ("Esc", "Atrás"),
        ],
        WizardStep::Routing => {
            if state.active_routing_modal != RoutingModal::None {
                vec![
                    ("↑/↓", "Mover"),
                    ("Enter", "Confirmar"),
                    ("Esc/Q", "Cancelar"),
                ]
            } else {
                vec![
                    ("1/2/3", "Criterio"),
                    ("←/→", "Mover Tarjeta"),
                    ("C", "Menú Criterio"),
                    ("F", "Filtro Fechas"),
                    ("Enter", "Siguiente"),
                    ("Esc", "Atrás"),
                ]
            }
        }
        WizardStep::Deduplication => vec![
            ("D", "Duplicados On/Off"),
            ("P", "Revisión Profunda"),
            ("Enter", "Siguiente"),
        ],
        WizardStep::Filters => vec![
            ("T", "Throttling"),
            ("Enter", "Siguiente"),
            ("Esc", "Atrás"),
        ],
        WizardStep::Summary => vec![
            ("Enter", "Iniciar Operación"),
            ("Esc", "Atrás"),
            ("q", "Salir"),
        ],
        WizardStep::Execution => {
            if state.show_cancel_modal {
                vec![
                    ("←/→/Tab", "Elegir"),
                    ("Enter", "Confirmar"),
                    ("S", "Sí"),
                    ("N/Esc", "No"),
                ]
            } else if state.progress.graceful_cancelling {
                vec![("Espere...", "Desmontando PST con seguridad")]
            } else {
                let p_label = if state.progress.is_paused { "Reanudar" } else { "Pausar" };
                vec![("P", p_label), ("Esc/Q", "Parada Segura")]
            }
        }
        WizardStep::Completion => {
            if state.html_report_path.is_some() {
                vec![("O", "Abrir HTML"), ("H", "Regenerar HTML"), ("Enter/q", "Salir")]
            } else {
                vec![("H", "Informe HTML"), ("Enter/q", "Salir")]
            }
        }
        WizardStep::PstDetailView => vec![
            ("↑/↓", "Navegar"),
            ("Enter/E/→", "Desplegar"),
            ("←/Backspace", "Plegar"),
            ("Espacio", "Aislar Métricas"),
            ("Esc/Q", "Volver"),
        ],
        WizardStep::SplitSelect => vec![
            ("↑/↓", "Navegar"),
            ("Espacio", "Marcar"),
            ("A/D", "Todos/Ninguno"),
            ("Enter", "Continuar"),
            ("E", "Explorador"),
            ("Esc", "Menú"),
        ],
        WizardStep::SplitFilter => vec![
            ("Tab", "Años/Meses"),
            ("↑/↓", "Navegar"),
            ("Espacio", "Marcar"),
            ("A/N", "Todos/Ninguno"),
            ("1/2/T", "Semestres/Todos"),
            ("Enter", "Siguiente"),
            ("Esc", "Atrás"),
        ],
        WizardStep::SplitConfig => vec![
            ("1/2/3", "Partición"),
            ("M", "Copiar/Mover"),
            ("O", "Carpeta Destino"),
            ("Enter", "Siguiente"),
            ("Esc", "Atrás"),
        ],
        WizardStep::SplitSummary => vec![
            ("Enter", "Iniciar Separación"),
            ("Esc", "Atrás"),
            ("Q", "Salir"),
        ],
        WizardStep::SplitExecution => {
            if state.progress.graceful_cancelling {
                vec![("Espere...", "Desmontando PST con seguridad")]
            } else {
                let p_label = if state.progress.is_paused { "Reanudar" } else { "Pausar" };
                vec![("P", p_label), ("Esc/Q", "Parada Segura")]
            }
        }
        WizardStep::SplitCompletion => vec![
            ("O", "Abrir Carpeta"),
            ("Enter/Q", "Menú Principal"),
        ],
    };

    render_footer(f, chunks[2], &shortcuts);
}
