use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::{
    app::{AppState, ExplorerItemType},
    ui::{format::format_size_mb, theme::Theme},
};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let explorer = &state.explorer;
    let has_warning = explorer.warning_notice.is_some();

    let top_height = if has_warning { 6 } else { 4 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top_height), // Ubicación y alerta
            Constraint::Min(10),            // Lista interactiva de archivos y carpetas
            Constraint::Length(3),          // Barra de atajos y confirmación
        ])
        .split(area);

    // 1. Panel de Ubicación y Alerta
    let mut top_lines = Vec::new();

    if let Some(ref notice) = explorer.warning_notice {
        top_lines.push(Line::from(vec![
            Span::styled(" ▲ NOTICE: ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
            Span::styled(notice.as_str(), Style::default().fg(Theme::WARNING)),
        ]));
    }

    let current_path_str = if explorer.is_drives_view {
        "Drive Selection".to_string()
    } else {
        explorer.current_path.to_string_lossy().to_string()
    };

    top_lines.push(Line::from(vec![
        Span::styled("◈ Exploring: ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled(current_path_str, Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
    ]));

    let pst_count = explorer.entries.iter().filter(|e| e.item_type == ExplorerItemType::PstFile).count();
    let sel_count = explorer.entries.iter().filter(|e| e.item_type == ExplorerItemType::PstFile && e.selected).count();

    top_lines.push(Line::from(vec![
        Span::styled("Directory items: ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(format!("{}", explorer.entries.len()), Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled("  |  PST files detected: ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(format!("{}", pst_count), Style::default().fg(if pst_count > 0 { Theme::SUCCESS } else { Theme::TEXT_MUTED }).add_modifier(Modifier::BOLD)),
        Span::styled("  |  Selected: ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(format!("{}", sel_count), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
    ]));

    let top_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if has_warning { Theme::WARNING } else { Theme::ACCENT_PRIMARY }))
        .title(" File System Explorer ");

    let top_inner = top_block.inner(chunks[0]);
    f.render_widget(top_block, chunks[0]);
    f.render_widget(Paragraph::new(top_lines), top_inner);

    // 2. Tabla de navegación de Archivos / Carpetas / Discos
    let header_cells = ["", "Type", "Name", "Size", "Path"].iter().map(|h| {
        Cell::from(*h).style(Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD))
    });
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = explorer.entries.iter().enumerate().map(|(idx, entry)| {
        let is_focused = idx == explorer.selected_idx;

        let (icon, type_label, type_style, checkbox, size_str) = match entry.item_type {
            ExplorerItemType::ParentDir => (
                "▲",
                "[UP]   ",
                Style::default().fg(Theme::ACCENT_PRIMARY),
                "   ",
                "-".to_string(),
            ),
            ExplorerItemType::Drive => (
                "▤",
                "[DRIVE]",
                Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD),
                "   ",
                "-".to_string(),
            ),
            ExplorerItemType::Directory => (
                "▸",
                "[DIR]  ",
                Style::default().fg(Theme::WARNING),
                "   ",
                "-".to_string(),
            ),
            ExplorerItemType::PstFile => {
                let cb = if entry.selected { "[x]" } else { "[ ]" };
                let sz = entry.size_mb.map(format_size_mb).unwrap_or_else(|| "-".to_string());
                (
                    "✉",
                    "[PST]  ",
                    Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD),
                    cb,
                    sz,
                )
            }
        };

        let pointer = if is_focused { "▶" } else { " " };
        let pointer_cell = Cell::from(pointer).style(Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD));
        
        let type_cell = Cell::from(format!("{} {}", icon, type_label)).style(type_style);

        let name_styled = if is_focused {
            Span::styled(&entry.name, Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(&entry.name, Style::default().fg(Theme::TEXT_MAIN))
        };
        let name_cell = Cell::from(name_styled);

        let size_cell = Cell::from(size_str).style(Style::default().fg(Theme::TEXT_MUTED));

        let sel_or_path = if entry.item_type == ExplorerItemType::PstFile {
            checkbox.to_string()
        } else {
            entry.path.to_string_lossy().to_string()
        };
        let path_cell = Cell::from(sel_or_path).style(if entry.selected {
            Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_MUTED)
        });

        let row = Row::new(vec![pointer_cell, type_cell, name_cell, size_cell, path_cell]);

        if is_focused {
            row.style(Style::default().bg(Theme::BG_CARD).fg(Theme::TEXT_MAIN))
        } else {
            row
        }
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),  // Flecha cursor
            Constraint::Length(12), // Tipo
            Constraint::Length(35), // Nombre
            Constraint::Length(14), // Tamaño
            Constraint::Min(25),    // Ruta o selección
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
            .title(if explorer.is_drives_view {
                " Available Local Drives "
            } else {
                " Folder Contents "
            }),
    );

    f.render_widget(table, chunks[1]);

    // 3. Barra de atajos
    let is_on_pst = explorer.entries.get(explorer.selected_idx)
        .map(|e| e.item_type == ExplorerItemType::PstFile)
        .unwrap_or(false);

    let mut second_row = vec![
        Span::styled("[C] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        Span::styled("Use Folder   ", Style::default().fg(Theme::SUCCESS)),
        Span::styled("[B] ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Search (Drives)   ", Style::default().fg(Theme::BRAND_PRIMARY)),
    ];

    if is_on_pst {
        second_row.push(Span::styled("[D] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)));
        second_row.push(Span::styled("View PST Details   ", Style::default().fg(Theme::ACCENT_PRIMARY)));
    }

    second_row.push(Span::styled("[Esc] ", Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD)));
    second_row.push(Span::styled("Back to Menu", Style::default().fg(Theme::TEXT_MUTED)));

    let shortcuts = vec![
        Line::from(vec![
            Span::styled("[↑/↓] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Navigate  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[Enter] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Open folder/drive  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[Backspace] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Up level  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[Space] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Toggle PST  ", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
        Line::from(second_row),
    ];

    f.render_widget(Paragraph::new(shortcuts).alignment(Alignment::Center), chunks[2]);
}

