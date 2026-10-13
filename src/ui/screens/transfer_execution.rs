use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, Paragraph},
    Frame,
};

use crate::{
    app::{AppState, TransferMode},
    ui::{
        format::{format_duration_compact, format_size_mb},
        theme::Theme,
    },
};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let progress = &state.progress;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 1. Ribbon de Estado y Contexto
            Constraint::Length(5), // 2. Barra de Progreso
            Constraint::Min(8),    // 3. Panel de Métricas KPI y Log en Vivo
        ])
        .split(area);

    // =========================================================================
    // 1. RIBBON DE ESTADO Y CONTEXTO
    // =========================================================================
    let status_border_color = if progress.graceful_cancelling || progress.throttling_active {
        Theme::WARNING
    } else {
        Theme::ACCENT_SECONDARY
    };

    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(status_border_color))
        .title(" ◈ Mailbox Transfer Session Status ◈ ");

    let status_inner = status_block.inner(chunks[0]);
    f.render_widget(status_block, chunks[0]);

    let status_badge = if progress.graceful_cancelling {
        Span::styled(
            " ■ SAFE CANCELLATION IN PROGRESS ",
            Style::default()
                .bg(Theme::WARNING)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
    } else if progress.is_paused {
        Span::styled(
            " ❚❚ PAUSED [P: Resume] ",
            Style::default()
                .bg(Theme::WARNING)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
    } else if progress.throttling_active {
        Span::styled(
            " ▲ THROTTLING ACTIVE ",
            Style::default()
                .bg(Theme::WARNING)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " ▶ TRANSFERRING ",
            Style::default()
                .bg(Theme::SUCCESS)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
    };

    let src_name = state
        .get_transfer_source_mailbox()
        .map(|m| m.display_name.as_str())
        .unwrap_or("Source");
    let dst_name = state
        .get_transfer_target_mailbox()
        .map(|m| m.display_name.as_str())
        .unwrap_or("Target");

    let status_line = Line::from(vec![
        status_badge,
        Span::raw("  "),
        Span::styled("From: ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(src_name, Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("  ──▶  To: ", Style::default().fg(Theme::ACCENT_SECONDARY)),
        Span::styled(dst_name, Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(
            if state.transfer.transfer_mode == TransferMode::Move { "[Mode: MOVE]" } else { "[Mode: COPY]" },
            Style::default().fg(if state.transfer.transfer_mode == TransferMode::Move { Theme::DANGER } else { Theme::SUCCESS }).add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(Paragraph::new(status_line), status_inner);

    // =========================================================================
    // 2. BARRA DE PROGRESO DE TRANSFERENCIA
    // =========================================================================
    let (current, total) = if progress.current_pst_total > 0 {
        (progress.current_pst_items, progress.current_pst_total)
    } else if progress.global_items_total > 0 {
        (progress.global_items_processed, progress.global_items_total)
    } else {
        (progress.imported_count, progress.imported_count.max(1))
    };

    let percent = if total > 0 {
        ((current as f64 / total as f64) * 100.0).clamp(0.0, 100.0) as u16
    } else {
        0
    };

    let gauge_title = if progress.graceful_cancelling {
        " Cancelling transfer with safety (RemoveStore & flushing) "
    } else if progress.is_paused {
        " Transfer Paused - Press 'P' to resume "
    } else if !progress.current_pst_name.is_empty() {
        " Transferring Folder / Stream "
    } else {
        " Overall Transfer Progress "
    };

    let gauge_label = format!("{} / {} emails ({}%)", current, total, percent);
    let gauge_color = if progress.graceful_cancelling || progress.is_paused {
        Theme::WARNING
    } else {
        Theme::SUCCESS
    };

    let gauge_widget = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(gauge_color))
                .title(gauge_title),
        )
        .gauge_style(Style::default().fg(gauge_color).bg(Theme::BG_CARD))
        .percent(percent)
        .label(gauge_label);

    f.render_widget(gauge_widget, chunks[1]);

    // =========================================================================
    // 3. PANEL DIVIDIDO: MÉTRICAS KPI (IZQUIERDA) Y LOG EN VIVO (DERECHA)
    // =========================================================================
    let kpi_and_log_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(34), Constraint::Min(20)])
        .split(chunks[2]);

    // KPI METRICS
    let kpi_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Performance & Metrics ");

    let kpi_inner = kpi_block.inner(kpi_and_log_chunks[0]);
    f.render_widget(kpi_block, kpi_and_log_chunks[0]);

    let kpi_lines = vec![
        Line::from(vec![
            Span::styled("Transferred:    ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{} msgs", progress.imported_count),
                Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Speed:          ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{:.1} msg/s", progress.speed_mps),
                Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("ETA:            ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                if progress.eta_seconds > 0 {
                    format_duration_compact(progress.eta_seconds)
                } else {
                    "--:--".to_string()
                },
                Style::default().fg(Theme::ACCENT_SECONDARY).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Duplicates:     ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{} skipped", progress.duplicates_skipped),
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]),
        Line::from(vec![
            Span::styled("Errors:         ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", progress.error_count),
                if progress.error_count > 0 {
                    Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::SUCCESS)
                },
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Data Volume:    ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format_size_mb(progress.imported_bytes as f64 / (1024.0 * 1024.0)),
                Style::default().fg(Theme::TEXT_MAIN),
            ),
        ]),
        Line::from(vec![
            Span::styled("Throttling:     ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                if progress.throttling_active {
                    "Backoff Active"
                } else {
                    "Optimal"
                },
                Style::default()
                    .fg(if progress.throttling_active { Theme::WARNING } else { Theme::SUCCESS })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(kpi_lines), kpi_inner);

    // LIVE ACTIVITY LOGS
    let log_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER_INACTIVE))
        .title(" Live Telemetry Stream ");

    let log_inner = log_block.inner(kpi_and_log_chunks[1]);
    f.render_widget(log_block, kpi_and_log_chunks[1]);

    let max_lines = log_inner.height as usize;
    let log_lines: Vec<Line> = state
        .activity_log
        .iter()
        .rev()
        .take(max_lines)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|entry| {
            let style = if entry.contains("[ERROR]") || entry.contains("Error") {
                Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD)
            } else if entry.contains("[WARN]") || entry.contains("Throttling") {
                Style::default().fg(Theme::WARNING)
            } else if entry.contains("[DONE]") || entry.contains("Completado") {
                Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)
            } else if entry.contains("[PAUSE]") || entry.contains("[RESUME]") {
                Style::default().fg(Theme::ACCENT_SECONDARY).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_MUTED)
            };
            Line::from(Span::styled(entry.as_str(), style))
        })
        .collect();

    f.render_widget(Paragraph::new(log_lines), log_inner);

    // =========================================================================
    // 4. MODAL DE CONFIRMACIÓN DE PARADA SEGURA
    // =========================================================================
    if state.show_cancel_modal {
        render_cancel_confirmation_modal(f, area, state);
    }
}

fn render_cancel_confirmation_modal(f: &mut Frame, area: Rect, state: &AppState) {
    let modal_width = 62.min(area.width.saturating_sub(4));
    let modal_height = 9.min(area.height.saturating_sub(2));

    let x = area.x + (area.width.saturating_sub(modal_width)) / 2;
    let y = area.y + (area.height.saturating_sub(modal_height)) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    f.render_widget(Clear, modal_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD))
        .title(" ⚠ Safe Transfer Cancellation ⚠ ");

    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Mensaje
            Constraint::Length(1), // Espacio
            Constraint::Length(1), // Botones
        ])
        .split(inner);

    let msg = vec![
        Line::from(Span::styled(
            "Do you want to stop the mailbox transfer safely?",
            Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "Current item will finish, COM objects released, and no data corrupted.",
            Style::default().fg(Theme::TEXT_MUTED),
        )),
    ];
    f.render_widget(Paragraph::new(msg).alignment(Alignment::Center), chunks[0]);

    let (btn_yes_style, btn_no_style) = if state.cancel_modal_selected_yes {
        (
            Style::default().bg(Theme::DANGER).fg(Color::White).add_modifier(Modifier::BOLD),
            Style::default().fg(Theme::TEXT_MUTED),
        )
    } else {
        (
            Style::default().fg(Theme::TEXT_MUTED),
            Style::default().bg(Theme::SUCCESS).fg(Color::Black).add_modifier(Modifier::BOLD),
        )
    };

    let btn_yes_text = if state.cancel_modal_selected_yes { " ► [ Yes ] ◄ " } else { "   [ Yes ]   " };
    let btn_no_text = if !state.cancel_modal_selected_yes { " ► [ No ] ◄ " } else { "   [ No ]   " };

    let buttons_line = Line::from(vec![
        Span::styled(btn_yes_text, btn_yes_style),
        Span::raw("        "),
        Span::styled(btn_no_text, btn_no_style),
    ]);
    f.render_widget(Paragraph::new(buttons_line).alignment(Alignment::Center), chunks[2]);
}
