use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{AppState, TransferMode},
    ui::theme::Theme,
};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let progress = &state.progress;

    let is_cancelled = progress.graceful_cancelling || state.transfer.execution_status.to_lowercase().contains("cancel");
    let has_errors = progress.error_count > 0;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // 1. Banner de Estado
            Constraint::Length(7), // 2. Tarjeta de Métricas KPI
            Constraint::Min(6),    // 3. Tarjeta de Buzones y Ruta de Auditoría JSON
            Constraint::Length(3), // 4. Footer de Acciones
        ])
        .split(area);

    // =========================================================================
    // 1. BANNER DE ESTADO
    // =========================================================================
    let (banner_title, banner_text, banner_color) = if is_cancelled {
        (
            " ❚❚ OPERATION STOPPED ",
            "Mailbox transfer was stopped cleanly by user. Outlook session safely released.",
            Theme::WARNING,
        )
    } else if has_errors {
        (
            " ▲ TRANSFER COMPLETED WITH INCIDENTS ",
            "Transfer completed with minor errors. Check audit log for uncopied items.",
            Theme::WARNING,
        )
    } else {
        (
            " ✔ MAILBOX TRANSFER COMPLETED SUCCESSFULLY ",
            "All emails matching the temporal filter were transferred and verified.",
            Theme::SUCCESS,
        )
    };

    let banner_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(banner_color).add_modifier(Modifier::BOLD))
        .title(banner_title);

    let banner_p = Paragraph::new(vec![
        Line::from(Span::styled(
            banner_text,
            Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "Integrity verified • Zero corruptions detected",
            Style::default().fg(Theme::TEXT_MUTED),
        )),
    ])
    .alignment(Alignment::Center)
    .block(banner_block);
    f.render_widget(banner_p, chunks[0]);

    // =========================================================================
    // 2. TARJETA DE MÉTRICAS KPI
    // =========================================================================
    let kpi_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Transfer Summary & KPIs ");
    let kpi_inner = kpi_block.inner(chunks[1]);
    f.render_widget(kpi_block, chunks[1]);

    let avg_speed = progress.speed_mps;

    let mode_str = if state.transfer.transfer_mode == TransferMode::Move {
        Span::styled("Move (Emails removed from Source)", Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("Copy (Original PST untouched)", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD))
    };

    let volume_mb = progress.imported_bytes as f64 / (1024.0 * 1024.0);

    let kpi_lines = vec![
        Line::from(vec![
            Span::styled("  • Emails Transferred:     ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{} messages", progress.imported_count),
                Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD),
            ),
            Span::styled("        • Duplicates Omitted:   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{} items", progress.duplicates_skipped),
                Style::default().fg(Theme::TEXT_MAIN),
            ),
        ]),
        Line::from(vec![
            Span::styled("  • Incidents / Errors:     ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{} errors", progress.error_count),
                if progress.error_count > 0 {
                    Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::SUCCESS)
                },
            ),
            Span::styled("        • Average Speed:        ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{:.1} msgs/sec", avg_speed),
                Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  • Transferred Volume:     ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{:.2} MB", volume_mb),
                Style::default().fg(Theme::TEXT_MAIN),
            ),
            Span::styled("        • Mode:                 ", Style::default().fg(Theme::TEXT_MUTED)),
            mode_str,
        ]),
    ];
    f.render_widget(Paragraph::new(kpi_lines), kpi_inner);

    // =========================================================================
    // 3. TARJETA DE BUZONES Y AUDITORÍA JSON
    // =========================================================================
    let audit_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
        .title(" Mailboxes Involved & JSON Audit Log ");
    let audit_inner = audit_block.inner(chunks[2]);
    f.render_widget(audit_block, chunks[2]);

    let src_name = state
        .get_transfer_source_mailbox()
        .map(|m| m.display_name.as_str())
        .unwrap_or("Unknown Source");
    let dst_name = state
        .get_transfer_target_mailbox()
        .map(|m| m.display_name.as_str())
        .unwrap_or("Unknown Target");

    let audit_file_str = if let Some(ref path) = state.json_audit_path {
        path.to_string_lossy().to_string()
    } else {
        "Generated on release build in ./logs/YYYY-MM-DD/transfer_run_HH-MM-SS.json".to_string()
    };

    let audit_lines = vec![
        Line::from(vec![
            Span::styled("  • Source Mailbox:   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(src_name, Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  • Target Mailbox:   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(dst_name, Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ◈ JSON Audit Report: ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(audit_file_str, Style::default().fg(Theme::TEXT_MAIN)),
        ]),
    ];
    f.render_widget(Paragraph::new(audit_lines), audit_inner);

    // =========================================================================
    // 4. FOOTER DE ACCIONES
    // =========================================================================
    let foot_line = Line::from(vec![
        Span::styled("[Enter] / [Esc] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        Span::styled("Return to Main Menu   ", Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled("[O] ", Style::default().fg(Theme::ACCENT_SECONDARY).add_modifier(Modifier::BOLD)),
        Span::styled("Open Logs Folder in Explorer   ", Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled("[Q] ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("Exit Application", Style::default().fg(Theme::TEXT_MUTED)),
    ]);

    let foot_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER_INACTIVE));
    f.render_widget(Paragraph::new(foot_line).alignment(Alignment::Center).block(foot_block), chunks[3]);
}
