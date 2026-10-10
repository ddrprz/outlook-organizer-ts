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
            Constraint::Length(6), // Deduplicación básica
            Constraint::Length(7), // Revisión profunda (Deep scan)
            Constraint::Min(2),    // Métodos de detección
        ])
        .split(area);

    // 1. Deduplicación
    let dedup_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Duplicate Detection & Omission ");

    let dedup_inner = dedup_block.inner(chunks[0]);
    f.render_widget(dedup_block, chunks[0]);

    let dedup_label = if state.deduplication_enabled {
        Span::styled("[x] SKIP DUPLICATES ENABLED — Do not import existing emails", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[ ] SKIP DUPLICATES DISABLED — Import all without checking duplicates", Style::default().fg(Theme::WARNING))
    };

    let dedup_lines = vec![
        Line::from(dedup_label),
        Line::from(""),
        Line::from(vec![
            Span::styled("Criteria: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("Message-ID (RFC 822) + MAPI SearchKey + Composite Key", Style::default().fg(Theme::TEXT_MAIN)),
        ]),
        Line::from(Span::styled("[D] Toggle Deduplication On/Off", Style::default().fg(Theme::ACCENT_PRIMARY))),
    ];
    f.render_widget(Paragraph::new(dedup_lines), dedup_inner);

    // 2. Revisión Profunda
    let deep_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
        .title(" Deep Scan ");

    let deep_inner = deep_block.inner(chunks[1]);
    f.render_widget(deep_block, chunks[1]);

    let deep_label = if state.deep_scan_enabled {
        Span::styled("[x] DEEP SCAN ENABLED (Recursively indexes subfolders)", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[ ] DEEP SCAN DISABLED (Only checks direct target folder)", Style::default().fg(Theme::TEXT_MUTED))
    };

    let deep_lines = vec![
        Line::from(deep_label),
        Line::from(""),
        Line::from(Span::styled("Detects emails that users moved manually to subfolders.", Style::default().fg(Theme::TEXT_MUTED))),
        Line::from(Span::styled("More thorough and safe against scattered duplicates.", Style::default().fg(Theme::BRAND_PRIMARY))),
        Line::from(Span::styled("[P] Toggle Deep Scan", Style::default().fg(Theme::ACCENT_PRIMARY))),
    ];
    f.render_widget(Paragraph::new(deep_lines), deep_inner);

    // 3. Resumen
    let summary = Line::from(vec![
        Span::styled("Algorithm: ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("High-speed in-memory SHA-256 hash comparison.", Style::default().fg(Theme::TEXT_MAIN)),
    ]);
    f.render_widget(Paragraph::new(summary).alignment(Alignment::Center), chunks[2]);
}
