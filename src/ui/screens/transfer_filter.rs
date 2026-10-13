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

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header banner
            Constraint::Length(5), // Section 0: Transfer Mode (Copy vs Move)
            Constraint::Min(8),    // Section 1 & 2: Years and Months side-by-side
            Constraint::Length(5), // Section 3: Folders to include
            Constraint::Length(3), // Footer key hints
        ])
        .split(area);

    // =========================================================================
    // 1. HEADER BANNER
    // =========================================================================
    let src_name = state
        .get_transfer_source_mailbox()
        .map(|m| m.display_name.as_str())
        .unwrap_or("None");
    let dst_name = state
        .get_transfer_target_mailbox()
        .map(|m| m.display_name.as_str())
        .unwrap_or("None");

    let header_line = Line::from(vec![
        Span::styled("Transfer Routing: ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(src_name, Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("  ──▶  ", Style::default().fg(Theme::ACCENT_SECONDARY).add_modifier(Modifier::BOLD)),
        Span::styled(dst_name, Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
    ]);

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
        .title(" Step 2 of 4 • Temporal Filters & Transfer Mode ");
    f.render_widget(Paragraph::new(header_line).block(header_block).alignment(Alignment::Center), chunks[0]);

    // =========================================================================
    // 2. MODO DE TRANSFERENCIA (COPIAR vs MOVER)
    // =========================================================================
    let mode_focused = transfer.filter_section_cursor == 0;
    let mode_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if mode_focused {
            Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::BORDER_INACTIVE)
        })
        .title(" [1] Transfer Mode (Space/Enter to toggle) ");

    let is_copy = transfer.transfer_mode == TransferMode::Copy;
    let mode_lines = vec![
        Line::from(vec![
            Span::styled(
                if is_copy { " (●) Copy Emails [Recommended] " } else { " (○) Copy Emails [Recommended] " },
                if is_copy {
                    Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_MUTED)
                },
            ),
            Span::styled("─ Original mailbox remains intact (Safe)", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
        Line::from(vec![
            Span::styled(
                if !is_copy { " (●) Move Emails [Destructive] " } else { " (○) Move Emails [Destructive] " },
                if !is_copy {
                    Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_MUTED)
                },
            ),
            Span::styled("─ Emails will be deleted from source after copying", Style::default().fg(Theme::DANGER)),
        ]),
    ];
    f.render_widget(Paragraph::new(mode_lines).block(mode_block), chunks[1]);

    // =========================================================================
    // 3. FILTROS DE AÑO Y MES (LADO A LADO)
    // =========================================================================
    let middle_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    // Panel de Años
    let year_focused = transfer.filter_section_cursor == 1;
    let year_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if year_focused {
            Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::BORDER_INACTIVE)
        })
        .title(" [2] Filter by Year (Source Mailbox) ");

    let mut year_lines = Vec::new();
    let all_y_radio = if transfer.all_years { "[x] All Available Years" } else { "[ ] All Available Years" };
    let all_y_style = if transfer.all_years {
        Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Theme::TEXT_MAIN)
    };
    year_lines.push(Line::from(vec![
        Span::styled(if year_focused && transfer.year_cursor == 0 { "▶ " } else { "  " }, Style::default().fg(Theme::ACCENT_PRIMARY)),
        Span::styled(all_y_radio, all_y_style),
        Span::styled("  (Press 'A' to toggle)", Style::default().fg(Theme::TEXT_MUTED)),
    ]));

    let years_to_display: Vec<u32> = if !transfer.available_years.is_empty() {
        transfer.available_years.clone()
    } else {
        vec![2026, 2025, 2024, 2023, 2022, 2021, 2020]
    };

    for (idx, y) in years_to_display.iter().enumerate() {
        let is_cur = year_focused && transfer.year_cursor == idx + 1;
        let is_sel = transfer.all_years || transfer.selected_years.contains(y);
        let check = if is_sel { format!("[x] Year {}", y) } else { format!("[ ] Year {}", y) };
        let style = if is_sel {
            Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_MUTED)
        };
        year_lines.push(Line::from(vec![
            Span::styled(if is_cur { "▶ " } else { "  " }, Style::default().fg(Theme::ACCENT_PRIMARY)),
            Span::styled(check, style),
        ]));
    }
    f.render_widget(Paragraph::new(year_lines).block(year_block), middle_split[0]);

    // Panel de Meses
    let month_focused = transfer.filter_section_cursor == 2;
    let month_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if month_focused {
            Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::BORDER_INACTIVE)
        })
        .title(" [3] Filter by Month ");

    let month_names = [
        "01 - January", "02 - February", "03 - March", "04 - April",
        "05 - May", "06 - June", "07 - July", "08 - August",
        "09 - September", "10 - October", "11 - November", "12 - December",
    ];

    let mut month_lines = Vec::new();
    let all_m_radio = if transfer.all_months { "[x] All Months" } else { "[ ] All Months" };
    let all_m_style = if transfer.all_months {
        Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Theme::TEXT_MAIN)
    };
    month_lines.push(Line::from(vec![
        Span::styled(if month_focused && transfer.month_cursor == 0 { "▶ " } else { "  " }, Style::default().fg(Theme::ACCENT_PRIMARY)),
        Span::styled(all_m_radio, all_m_style),
    ]));

    for (idx, name) in month_names.iter().enumerate() {
        let m_num = (idx + 1) as u32;
        let is_cur = month_focused && transfer.month_cursor == idx + 1;
        let is_sel = transfer.all_months || transfer.selected_months.contains(&m_num);
        let check = if is_sel { format!("[x] {}", name) } else { format!("[ ] {}", name) };
        let style = if is_sel {
            Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_MUTED)
        };
        month_lines.push(Line::from(vec![
            Span::styled(if is_cur { "▶ " } else { "  " }, Style::default().fg(Theme::ACCENT_PRIMARY)),
            Span::styled(check, style),
        ]));
    }
    f.render_widget(Paragraph::new(month_lines).block(month_block), middle_split[1]);

    // =========================================================================
    // 4. CARPETAS A INCLUIR
    // =========================================================================
    let folder_focused = transfer.filter_section_cursor == 3;
    let folder_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if folder_focused {
            Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::BORDER_INACTIVE)
        })
        .title(" [4] Folder Scope ");

    let folder_opts = [
        (transfer.include_inbox, "Inbox / Bandeja de entrada"),
        (transfer.include_sent, "Sent Items / Elementos enviados"),
        (transfer.include_deleted, "Deleted Items / Elementos eliminados"),
        (transfer.include_custom_folders, "Custom Subfolders / Carpetas personalizadas"),
    ];

    let mut folder_spans = Vec::new();
    for (idx, (active, label)) in folder_opts.iter().enumerate() {
        let is_cur = folder_focused && transfer.folder_cursor == idx;
        let check = if *active { format!("[x] {}   ", label) } else { format!("[ ] {}   ", label) };
        folder_spans.push(Span::styled(if is_cur { "▶ " } else { "  " }, Style::default().fg(Theme::ACCENT_PRIMARY)));
        folder_spans.push(Span::styled(
            check,
            if *active {
                Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_MUTED)
            },
        ));
    }
    f.render_widget(Paragraph::new(Line::from(folder_spans)).block(folder_block), chunks[3]);

    // =========================================================================
    // 5. FOOTER DE ATAJOS
    // =========================================================================
    let foot_line = Line::from(vec![
        Span::styled("[Tab/1-4] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Switch Section   ", Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled("[↑/↓] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Navigate   ", Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled("[Space] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        Span::styled("Toggle   ", Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled("[A] ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("All Years/Months   ", Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled("[Enter] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        Span::styled("Review Summary   ", Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled("[Esc] ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("Back", Style::default().fg(Theme::TEXT_MUTED)),
    ]);

    let foot_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER_INACTIVE));
    f.render_widget(Paragraph::new(foot_line).alignment(Alignment::Center).block(foot_block), chunks[4]);
}
