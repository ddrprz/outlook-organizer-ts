use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Row, Table},
    Frame,
};

use crate::{app::AppState, ui::theme::Theme};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Encabezado de estado
            Constraint::Min(8),    // Tabla de archivos PST generados
            Constraint::Length(4), // Botones de acción
        ])
        .split(area);

    // 1. Encabezado de estado
    let is_aborted = state.split.execution_status.contains("aborted") || state.split.execution_status.contains("cancel");
    let (status_title, status_color) = if is_aborted {
        ("PROCESS SAFELY ABORTED BY USER", Theme::WARNING)
    } else {
        ("OPERATION COMPLETED SUCCESSFULLY", Theme::SUCCESS)
    };

    let summary_lines = vec![
        Line::from(vec![
            Span::styled(format!("✓ {}", status_title), Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" — Total emails transferred: {}", state.split.total_extracted), Style::default().fg(Theme::TEXT_MAIN)),
        ]),
        Line::from(vec![
            Span::styled(format!("{} PST file(s) created in: \"{}\"", state.split.generated_psts.len(), state.split.output_dir), Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];
    let summary_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(status_color))
        .title(" Split Status ");
    f.render_widget(Paragraph::new(summary_lines).block(summary_block), chunks[0]);

    // 2. Tabla de Archivos PST Generados
    let rows: Vec<Row> = if state.split.generated_psts.is_empty() {
        vec![Row::new(vec![
            Span::styled("No new files were generated or operation was cancelled early.", Style::default().fg(Theme::WARNING)),
            Span::raw(""),
            Span::raw(""),
            Span::raw(""),
        ])]
    } else {
        state
            .split
            .generated_psts
            .iter()
            .enumerate()
            .map(|(i, pst)| {
                Row::new(vec![
                    Span::styled(format!(" {:2}. {}", i + 1, pst.file_name), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{} emails", pst.items_count), Style::default().fg(Theme::SUCCESS)),
                    Span::styled(format!("{:.2} MB", pst.size_mb), Style::default().fg(Theme::ACCENT_PRIMARY)),
                    Span::styled(pst.file_path.clone(), Style::default().fg(Theme::TEXT_MUTED)),
                ])
            })
            .collect()
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(32), // Name
            Constraint::Length(16), // Emails
            Constraint::Length(14), // Size
            Constraint::Min(25),    // Path
        ],
    )
    .header(
        Row::new(vec![
            Span::styled(" Generated PST File", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" Emails", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" Size", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" Disk Location", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        ])
        .style(Style::default().bg(Theme::SURFACE)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
            .title(format!(" Resulting PST Files ({}) ", state.split.generated_psts.len())),
    );

    f.render_widget(table, chunks[1]);

    // 3. Botones de acción
    let btn_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  [ O: OPEN FOLDER IN WINDOWS ]  ", Style::default().bg(Theme::ACCENT_PRIMARY).fg(Theme::BG_DARK).add_modifier(Modifier::BOLD)),
            Span::raw("    "),
            Span::styled("  [ ENTER / Q: RETURN TO MAIN MENU ]  ", Style::default().bg(Theme::SURFACE).fg(Theme::TEXT_MAIN)),
        ]),
    ];
    f.render_widget(Paragraph::new(btn_lines).alignment(Alignment::Center), chunks[2]);
}
