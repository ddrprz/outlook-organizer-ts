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
    let transfer = &state.transfer;
    let is_move = transfer.transfer_mode == TransferMode::Move;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header banner
            Constraint::Length(6), // Dual mailbox review cards
            Constraint::Min(8),    // Configuration & Filters review
            if is_move { Constraint::Length(4) } else { Constraint::Length(0) }, // Warning box if Move
            Constraint::Length(3), // Footer confirmation
        ])
        .split(area);

    // =========================================================================
    // 1. HEADER BANNER
    // =========================================================================
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
        .title(" Step 3 of 4 • Pre-Flight Transfer Summary ");
    let title_line = Line::from(vec![
        Span::styled("Please review the transfer parameters before proceeding. ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("Press [Enter] to launch transfer.", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
    ]);
    f.render_widget(Paragraph::new(title_line).alignment(Alignment::Center).block(title_block), chunks[0]);

    // =========================================================================
    // 2. TARJETAS DE ORIGEN Y DESTINO
    // =========================================================================
    let mbox_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let src = state.get_transfer_source_mailbox();
    let src_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BRAND_PRIMARY))
        .title(" Source Mailbox (Extract) ");
    let src_inner = src_block.inner(mbox_split[0]);
    f.render_widget(src_block, mbox_split[0]);

    let src_lines = if let Some(s) = src {
        vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(&s.display_name, Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Type: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(&s.store_type, Style::default().fg(Theme::ACCENT_PRIMARY)),
                Span::styled("  •  Used: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(format!("{:.1} GB", s.get_used_gb()), Style::default().fg(Theme::TEXT_MAIN)),
            ]),
        ]
    } else {
        vec![Line::from(Span::styled("No source selected", Style::default().fg(Theme::DANGER)))]
    };
    f.render_widget(Paragraph::new(src_lines), src_inner);

    let dst = state.get_transfer_target_mailbox();
    let dst_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::SUCCESS))
        .title(" Target Mailbox (Deposit) ");
    let dst_inner = dst_block.inner(mbox_split[1]);
    f.render_widget(dst_block, mbox_split[1]);

    let dst_lines = if let Some(d) = dst {
        vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(&d.display_name, Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Type: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(&d.store_type, Style::default().fg(Theme::ACCENT_PRIMARY)),
                Span::styled("  •  Free: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(format!("{:.1} GB / {}", d.get_free_gb(), d.get_quota_display()), Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
            ]),
        ]
    } else {
        vec![Line::from(Span::styled("No target selected", Style::default().fg(Theme::DANGER)))]
    };
    f.render_widget(Paragraph::new(dst_lines), dst_inner);

    // =========================================================================
    // 3. PARÁMETROS DE CONFIGURACIÓN Y FILTROS
    // =========================================================================
    let param_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Transfer Parameters & Filters ");
    let param_inner = param_block.inner(chunks[2]);
    f.render_widget(param_block, chunks[2]);

    let mode_str = if is_move {
        Span::styled("Move Emails [Destructive - Deletes from Source]", Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("Copy Emails [Safe - Original Untouched]", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD))
    };

    let years_str = if transfer.all_years {
        "All Available Years".to_string()
    } else if transfer.selected_years.is_empty() {
        "All Available Years (Default)".to_string()
    } else {
        transfer.selected_years.iter().map(|y| y.to_string()).collect::<Vec<_>>().join(", ")
    };

    let months_str = if transfer.all_months {
        "All Months (Jan - Dec)".to_string()
    } else if transfer.selected_months.is_empty() {
        "All Months (Default)".to_string()
    } else {
        transfer.selected_months.iter().map(|m| format!("{:02}", m)).collect::<Vec<_>>().join(", ")
    };

    let mut folders_list = Vec::new();
    if transfer.include_inbox { folders_list.push("Inbox"); }
    if transfer.include_sent { folders_list.push("Sent Items"); }
    if transfer.include_deleted { folders_list.push("Deleted Items"); }
    if transfer.include_custom_folders { folders_list.push("Custom Folders"); }
    let folders_str = if folders_list.is_empty() { "None selected".to_string() } else { folders_list.join(", ") };

    let param_lines = vec![
        Line::from(vec![
            Span::styled("  • Transfer Mode:          ", Style::default().fg(Theme::TEXT_MUTED)),
            mode_str,
        ]),
        Line::from(vec![
            Span::styled("  • Year Filter:            ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(years_str, Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  • Month Filter:           ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(months_str, Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  • Included Folders:       ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(folders_str, Style::default().fg(Theme::ACCENT_SECONDARY)),
        ]),
        Line::from(vec![
            Span::styled("  • Deduplication Policy:   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("Enabled (SearchKey / Message-ID hash match)", Style::default().fg(Theme::SUCCESS)),
        ]),
        Line::from(vec![
            Span::styled("  • Throttling & Backoff:   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("Adaptive MAPI Protection (Exchange Online)", Style::default().fg(Theme::ACCENT_PRIMARY)),
        ]),
    ];
    f.render_widget(Paragraph::new(param_lines), param_inner);

    // =========================================================================
    // 4. ADVERTENCIA EN MODO MOVER (SI APLICA)
    // =========================================================================
    if is_move {
        let warn_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::DANGER))
            .title(" ⚠ DESTRUCTIVE MODE WARNING ⚠ ");
        let warn_p = Paragraph::new(vec![
            Line::from(Span::styled("Emails will be moved and permanently deleted from the source mailbox.", Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("Make sure you have a valid backup before proceeding.", Style::default().fg(Theme::TEXT_MAIN))),
        ])
        .alignment(Alignment::Center)
        .block(warn_block);
        f.render_widget(warn_p, chunks[3]);
    }

    // =========================================================================
    // 5. FOOTER DE CONFIRMACIÓN
    // =========================================================================
    let footer_idx = if is_move { 4 } else { 3 };
    let foot_line = Line::from(vec![
        Span::styled("[Enter] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        Span::styled("START TRANSFER NOW   ", Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
        Span::styled("[Esc] ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("Back to Filters   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[Q] ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("Main Menu", Style::default().fg(Theme::TEXT_MUTED)),
    ]);

    let foot_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER_INACTIVE));
    f.render_widget(Paragraph::new(foot_line).alignment(Alignment::Center).block(foot_block), chunks[footer_idx]);
}
