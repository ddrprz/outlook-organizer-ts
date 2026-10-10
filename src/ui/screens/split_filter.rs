use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{app::AppState, ui::theme::Theme};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Guía superior
            Constraint::Min(12),   // Paneles de Años y Meses
            Constraint::Length(5), // Carpetas a incluir
        ])
        .split(area);

    // 1. Guía superior
    let guide_lines = vec![
        Line::from(vec![
            Span::styled("PERIOD AND FOLDER FILTER", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" — Select the years and months you wish to extract into new PSTs", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
        Line::from(vec![
            Span::styled("[Tab] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Toggle Years/Months  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[Space] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Select/Deselect  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[A/N] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("All/None  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[1/2] ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Semesters  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[Enter] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
            Span::styled("Next", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];
    f.render_widget(Paragraph::new(guide_lines), chunks[0]);

    // 2. Paneles de Años y Meses (Split horizontal)
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Years
            Constraint::Percentage(50), // Months
        ])
        .split(chunks[1]);

    // Detalle en caché del PST de origen
    let pst_detail = state
        .split
        .source_pst
        .as_ref()
        .and_then(|p| state.pst_details_cache.get(&p.path));

    // PANEL IZQUIERDO: AÑOS
    let mut year_lines = Vec::new();
    year_lines.push(Line::from(""));

    let available_years = &state.split.available_years;
    for (i, &y) in available_years.iter().enumerate() {
        let is_cursor = state.split.year_cursor == i;
        let is_selected = state.split.selected_years.contains(&y);

        let check_box = if is_selected { "[x] " } else { "[ ] " };
        let count_str = if let Some(det) = pst_detail {
            det.counts_by_year
                .get(&y.to_string())
                .map(|cnt| format!(" ({} emails)", cnt))
                .unwrap_or_else(|| " (0 emails)".to_string())
        } else {
            String::new()
        };

        let pointer = if is_cursor { "▸ " } else { "  " };

        let line_style = if is_cursor {
            Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default().fg(Theme::TEXT_MAIN)
        } else {
            Style::default().fg(Theme::TEXT_MUTED)
        };

        year_lines.push(Line::from(vec![
            Span::styled(pointer, Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(check_box, if is_selected { Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
            Span::styled(format!("Year {}", y), line_style),
            Span::styled(count_str, Style::default().fg(Theme::TEXT_MUTED)),
        ]));
    }

    if available_years.is_empty() {
        year_lines.push(Line::from(Span::styled("No years detected. Full file will be processed.", Style::default().fg(Theme::WARNING))));
    }

    let years_count_selected = state.split.selected_years.len();
    let years_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if state.split.config_cursor == 0 { Theme::ACCENT_PRIMARY } else { Theme::BORDER_INACTIVE }))
        .title(format!(" Selected Years ({}/{}) [A: All / N: None] ", years_count_selected, available_years.len()));
    f.render_widget(Paragraph::new(year_lines).block(years_block), body_chunks[0]);

    // PANEL DERECHO: MESES (Solo los meses que realmente existen en el PST para los años seleccionados)
    let mut month_lines = Vec::new();
    month_lines.push(Line::from(""));

    let available_months = &state.split.available_months;
    let target_years: Vec<String> = if state.split.selected_years.is_empty() {
        state.split.available_years.iter().map(|y| y.to_string()).collect()
    } else {
        state.split.selected_years.iter().map(|y| y.to_string()).collect()
    };

    for (i, &m_num) in available_months.iter().enumerate() {
        let is_cursor = state.split.month_cursor == i;
        let is_selected = state.split.selected_months.contains(&m_num);

        let check_box = if is_selected { "[x] " } else { "[ ] " };
        let pointer = if is_cursor { "▸ " } else { "  " };

        let m_name = match m_num {
            1 => "01 - January",
            2 => "02 - February",
            3 => "03 - March",
            4 => "04 - April",
            5 => "05 - May",
            6 => "06 - June",
            7 => "07 - July",
            8 => "08 - August",
            9 => "09 - September",
            10 => "10 - October",
            11 => "11 - November",
            12 => "12 - December",
            _ => "Unknown Month",
        };

        let count_str = if let Some(det) = pst_detail {
            let cnt: usize = target_years
                .iter()
                .filter_map(|y| {
                    let ym_key = format!("{}-{:02}", y, m_num);
                    det.counts_by_month.get(&ym_key).copied()
                })
                .sum();
            if cnt > 0 {
                format!(" ({} emails)", cnt)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let line_style = if is_cursor {
            Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default().fg(Theme::TEXT_MAIN)
        } else {
            Style::default().fg(Theme::TEXT_MUTED)
        };

        month_lines.push(Line::from(vec![
            Span::styled(pointer, Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(check_box, if is_selected { Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
            Span::styled(m_name, line_style),
            Span::styled(count_str, Style::default().fg(Theme::TEXT_MUTED)),
        ]));
    }

    if available_months.is_empty() {
        month_lines.push(Line::from(Span::styled("No months with emails found for selected years.", Style::default().fg(Theme::WARNING))));
    }

    let months_count_selected = state.split.selected_months.len();
    let months_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if state.split.config_cursor == 1 { Theme::ACCENT_PRIMARY } else { Theme::BORDER_INACTIVE }))
        .title(format!(" Selected Months ({}/{}) [1: H1 / 2: H2 / A: All] ", months_count_selected, available_months.len()));
    f.render_widget(Paragraph::new(month_lines).block(months_block), body_chunks[1]);

    // 3. Panel Inferior: Carpetas a Incluir
    let folders_line = vec![
        Span::styled("Source folders to extract:  ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(if state.split.include_inbox { "[x] " } else { "[ ] " }, if state.split.include_inbox { Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
        Span::styled("Inbox [I]    ", Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled(if state.split.include_sent { "[x] " } else { "[ ] " }, if state.split.include_sent { Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
        Span::styled("Sent Items [S]    ", Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled(if state.split.include_deleted { "[x] " } else { "[ ] " }, if state.split.include_deleted { Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
        Span::styled("Deleted Items [D]    ", Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled(if state.split.include_custom_folders { "[x] " } else { "[ ] " }, if state.split.include_custom_folders { Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
        Span::styled("Custom Folders [C]", Style::default().fg(Theme::TEXT_MAIN)),
    ];

    let folders_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER_INACTIVE))
        .title(" Folder Scope ");
    let folders_p = Paragraph::new(vec![Line::from(""), Line::from(folders_line)]).block(folders_block);
    f.render_widget(folders_p, chunks[2]);
}
