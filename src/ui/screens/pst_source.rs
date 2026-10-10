use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::{
    app::AppState,
    ui::{format::format_size_mb, theme::Theme},
};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let has_warning = state.pst_warning_notice.is_some();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if has_warning {
            vec![
                Constraint::Length(5),  // Modo de escaneo y ruta
                Constraint::Min(8),     // Tabla de PSTs encontrados
                Constraint::Length(2),  // Advertencia
                Constraint::Length(2),  // Atajos rápidos
            ]
        } else {
            vec![
                Constraint::Length(5),  // Modo de escaneo y ruta
                Constraint::Min(8),     // Tabla de PSTs encontrados
                Constraint::Length(2),  // Atajos rápidos
            ]
        })
        .split(area);

    // 1. Selector de Origen / Ruta
    let source_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Search Location ");

    let source_inner = source_block.inner(chunks[0]);
    f.render_widget(source_block, chunks[0]);

    let source_lines = vec![
        Line::from(vec![
            Span::styled("Current scan path: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!("▸ {}", state.pst_scan_path), Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" (Default path C:\\Correo)", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
        Line::from(vec![
            Span::styled("Detected files: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!("{}", state.discovered_psts.len()), Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
            Span::styled(" | Selected for import: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", state.discovered_psts.iter().filter(|p| p.selected).count()),
                Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(source_lines), source_inner);

    // 2. Tabla de PSTs interactiva
    let header_cells = ["Sel", "File Name", "Size (MB)", "Full Path"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = state.discovered_psts.iter().enumerate().map(|(idx, item)| {
        let is_focused = idx == state.selected_pst_table_idx;
        let checkbox = if item.selected { "[x]" } else { "[ ]" };
        let sel_cell = Cell::from(checkbox).style(if item.selected {
            Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_MUTED)
        });

        let name_cell = Cell::from(item.name.as_str());
        let size_cell = Cell::from(format_size_mb(item.size_mb));
        let path_cell = Cell::from(item.path.as_str());

        let row = Row::new(vec![sel_cell, name_cell, size_cell, path_cell]);
        if is_focused {
            row.style(Style::default().bg(Theme::BG_CARD).fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD))
        } else {
            row
        }
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Length(25),
            Constraint::Length(15),
            Constraint::Min(30),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
            .title(" Discovered PST Files (Press Space to toggle items to import) "),
    );

    f.render_widget(table, chunks[1]);

    // 3. Advertencia si aplica
    let help_idx = if has_warning {
        if let Some(ref notice) = state.pst_warning_notice {
            let warn_p = Paragraph::new(Line::from(vec![
                Span::styled("▲  ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
                Span::styled(notice.as_str(), Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
            ]))
            .alignment(Alignment::Center);
            f.render_widget(warn_p, chunks[2]);
        }
        3
    } else {
        2
    };

    // 4. Atajos
    let help_line = Line::from(vec![
        Span::styled("[↑/↓] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Navigate row   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[Space] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Toggle   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[D] ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("View PST Details   ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("[E] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Explore   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[A/N] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("All/None   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[Enter] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        Span::styled("Continue", Style::default().fg(Theme::SUCCESS)),
    ]);
    f.render_widget(Paragraph::new(help_line).alignment(Alignment::Center), chunks[help_idx]);
}
