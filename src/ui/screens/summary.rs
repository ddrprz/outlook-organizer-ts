use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{AppState, RoutingGranularity, TransferMode},
    ui::{format::format_size_mb, theme::Theme},
};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(13),   // Tarjeta Resumen Consolidado
            Constraint::Length(4), // Botones de Confirmación
        ])
        .split(area);

    // 1. Resumen Consolidado
    let summary_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Pre-Flight Configuration Summary ");

    let summary_inner = summary_block.inner(chunks[0]);
    f.render_widget(summary_block, chunks[0]);

    let selected_psts_count = state.discovered_psts.iter().filter(|p| p.selected).count();
    let total_size_mb: f64 = state.discovered_psts.iter().filter(|p| p.selected).map(|p| p.size_mb).sum();

    let mode_str = match state.transfer_mode {
        TransferMode::Copy => "COPY (Non-destructive: original PST intact on disk)",
        TransferMode::Move => "MOVE (Destructive: emails deleted from PST after verified transfer)",
    };
    let mode_color = match state.transfer_mode {
        TransferMode::Copy => Theme::SUCCESS,
        TransferMode::Move => Theme::DANGER,
    };

    let (routing_criterion_str, routing_criterion_color) = match state.routing_granularity {
        RoutingGranularity::Mirror => (
            "[1] Preserve Original Structure (Native PST - Direct 1:1 mapping)",
            Theme::BRAND_PRIMARY,
        ),
        RoutingGranularity::Years => (
            "[2] Group by Year (Hierarchical: [Mailbox] / <Year> / <Folders>)",
            Theme::ACCENT_PRIMARY,
        ),
        RoutingGranularity::YearsAndMonths => (
            "[3] Group by Year & Month (Hierarchical: [Mailbox] / <Year> / <Month> / <Folders>)",
            Theme::SUCCESS,
        ),
    };

    let (date_filter_str, date_filter_color) = if !state.routing_all_years || !state.routing_all_months {
        let years_str = state.format_years_filter_display();
        let months_str = state.format_months_filter_display();
        (
            format!("Active Filter: {} | {}", years_str, months_str),
            Theme::WARNING,
        )
    } else {
        (
            "Full History (No date filter: all years and months)".to_string(),
            Theme::TEXT_MAIN,
        )
    };

    let selected_folders = state.folder_tree.selected_paths();
    let folder_filter_str = if selected_folders.is_empty() {
        "All standard folders in PST".to_string()
    } else {
        let count = selected_folders.len();
        if count <= 3 {
            format!("{} selected folders ({})", count, selected_folders.join(", "))
        } else {
            let sample = selected_folders[..2].join(", ");
            format!("{} selected folders (includes {}, ...)", count, sample)
        }
    };

    let selected_mailboxes = state.selected_mailboxes();
    let mailbox_summary_str = if selected_mailboxes.is_empty() {
        "None selected (▲ At least one required)".to_string()
    } else if selected_mailboxes.len() == 1 {
        format!("{} ({})", selected_mailboxes[0].display_name, selected_mailboxes[0].store_type)
    } else {
        let names = selected_mailboxes.iter().map(|m| m.display_name.as_str()).collect::<Vec<_>>().join(", ");
        format!("{} mailboxes selected: {}", selected_mailboxes.len(), names)
    };

    let summary_lines = vec![
        Line::from(vec![
            Span::styled("• Outlook MAPI Profile: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                if state.use_default_profile { "Windows Default" } else { &state.custom_profile_name },
                Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("• Source PSTs:          ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{} files selected ({})", selected_psts_count, format_size_mb(total_size_mb)),
                Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("• Target Mailbox(es):   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(mailbox_summary_str, Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("• Transfer Mode:        ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!(" {}", mode_str), Style::default().fg(mode_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("• Routing Criterion:    ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!(" {}", routing_criterion_str),
                Style::default().fg(routing_criterion_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("• Date Filter:          ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!(" {}", date_filter_str),
                Style::default().fg(date_filter_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("• Included Folders:     ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!(" {}", folder_filter_str), Style::default().fg(Theme::TEXT_MAIN)),
        ]),
        Line::from(vec![
            Span::styled("• Deduplication:        ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(if state.deduplication_enabled { "Enabled (Message-ID + Key)" } else { "Disabled" }, Style::default().fg(Theme::TEXT_MAIN)),
            Span::styled(" | Deep Scan: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(if state.deep_scan_enabled { "Yes (Recursive)" } else { "No" }, Style::default().fg(Theme::TEXT_MAIN)),
        ]),
        Line::from(vec![
            Span::styled("• Adaptive Throttling:  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(if state.adaptive_throttling_enabled { "Enabled (M365 429 Protection)" } else { "Disabled" }, Style::default().fg(Theme::SUCCESS)),
        ]),
    ];
    f.render_widget(Paragraph::new(summary_lines), summary_inner);


    // 2. Botones de Confirmación
    let confirm_lines = vec![
        Line::from(vec![
            Span::styled("  [ ENTER: START IMPORT PROCESS ]  ", Style::default().bg(Theme::SUCCESS).fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
            Span::styled("     ", Style::default()),
            Span::styled("  [ ESC: GO BACK ]  ", Style::default().bg(Theme::BG_CARD).fg(Theme::TEXT_MAIN)),
        ]),
    ];
    f.render_widget(Paragraph::new(confirm_lines).alignment(Alignment::Center), chunks[1]);
}
