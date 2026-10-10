use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
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
            Constraint::Length(6), // Filtros de Fecha
            Constraint::Length(7), // Throttling Adaptativo
            Constraint::Min(2),    // Recomendaciones M365
        ])
        .split(area);

    // 1. Filtros de Fecha
    let date_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Time Filters (Optional) ");

    let date_inner = date_block.inner(chunks[0]);
    f.render_widget(date_block, chunks[0]);

    let date_lines = vec![
        Line::from(vec![
            Span::styled("Import Range: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("Full History (No restrictive filter)", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(Span::styled("All emails contained in the selected PSTs will be imported.", Style::default().fg(Theme::TEXT_MAIN))),
        Line::from(Span::styled("[F] Change range to specific year", Style::default().fg(Theme::TEXT_MUTED))),
    ];
    f.render_widget(Paragraph::new(date_lines), date_inner);

    // 2. Throttling Adaptativo
    let thrott_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
        .title(" Performance Control: Adaptive Throttling ");

    let thrott_inner = thrott_block.inner(chunks[1]);
    f.render_widget(thrott_block, chunks[1]);

    let thrott_label = if state.adaptive_throttling_enabled {
        Span::styled("[x] ADAPTIVE THROTTLING ENABLED (Recommended)", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[ ] THROTTLING DISABLED (Fixed speed / Throttling risk)", Style::default().fg(Theme::WARNING))
    };

    let thrott_lines = vec![
        Line::from(thrott_label),
        Line::from(""),
        Line::from(Span::styled("Automatically regulates MAPI/COM call speed when Exchange", Style::default().fg(Theme::TEXT_MUTED))),
        Line::from(Span::styled("Online or Microsoft 365 detects high latency or emits HTTP 429 backoff.", Style::default().fg(Theme::TEXT_MUTED))),
        Line::from(Span::styled("[T] Toggle Adaptive Throttling", Style::default().fg(Theme::ACCENT_PRIMARY))),
    ];
    f.render_widget(Paragraph::new(thrott_lines), thrott_inner);

    // 3. Recomendación
    let rec = Line::from(vec![
        Span::styled("◈ M365 Best Practice: ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
        Span::styled("Keeping throttling active prevents tenant disconnections.", Style::default().fg(Theme::TEXT_MAIN)),
    ]);
    f.render_widget(Paragraph::new(rec).alignment(Alignment::Center), chunks[2]);
}
