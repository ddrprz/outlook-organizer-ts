use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{app::AppState, ui::theme::Theme};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let is_online = state.is_target_online();

    let chunks = if is_online {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // Alerta de Modo En Línea
                Constraint::Length(5), // Deduplicación básica
                Constraint::Length(6), // Revisión profunda (Deep scan)
                Constraint::Min(2),    // Métodos de detección
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Deduplicación básica
                Constraint::Length(7), // Revisión profunda (Deep scan)
                Constraint::Min(2),    // Métodos de detección
            ])
            .split(area)
    };

    let mut current_idx = 0;

    // 0. Banner de Advertencia si el Buzón Destino es en Línea
    if is_online {
        let warn_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::WARNING))
            .title(" ▲ ONLINE MAILBOX NOTICE (Exchange Online / Microsoft 365) ");

        let warn_inner = warn_block.inner(chunks[current_idx]);
        f.render_widget(warn_block, chunks[current_idx]);

        let warn_lines = vec![
            Line::from(vec![
                Span::styled("  ⚠ OPTIMIZATION NOTICE: ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
                Span::styled(
                    "Target mailbox operates in Cloud/Online Mode (no local .ost cache).",
                    Style::default().fg(Theme::TEXT_MAIN),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  • Deep Scan has been automatically disabled to prevent extreme WAN latency and 429 throttling.",
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  • Direct folder deduplication remains 100% ACTIVE and will safely prevent duplicates in target folders.",
                    Style::default().fg(Theme::SUCCESS),
                ),
            ]),
        ];
        f.render_widget(Paragraph::new(warn_lines), warn_inner);
        current_idx += 1;
    }

    // 1. Deduplicación
    let dedup_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Duplicate Detection & Omission ");

    let dedup_inner = dedup_block.inner(chunks[current_idx]);
    f.render_widget(dedup_block, chunks[current_idx]);

    let dedup_label = if state.deduplication_enabled {
        Span::styled("[x] SKIP DUPLICATES ENABLED — Do not import existing emails", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[ ] SKIP DUPLICATES DISABLED — Import all without checking duplicates", Style::default().fg(Theme::WARNING))
    };

    let dedup_lines = vec![
        Line::from(dedup_label),
        Line::from(vec![
            Span::styled("Criteria: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("Message-ID (RFC 822) + MAPI SearchKey + Composite Key", Style::default().fg(Theme::TEXT_MAIN)),
        ]),
        Line::from(Span::styled("[D] Toggle Deduplication On/Off", Style::default().fg(Theme::ACCENT_PRIMARY))),
    ];
    f.render_widget(Paragraph::new(dedup_lines), dedup_inner);
    current_idx += 1;

    // 2. Revisión Profunda
    let deep_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if is_online { Style::default().fg(Theme::TEXT_MUTED) } else { Style::default().fg(Theme::ACCENT_SECONDARY) })
        .title(if is_online { " Deep Scan (Auto-Adjusted) " } else { " Deep Scan " });

    let deep_inner = deep_block.inner(chunks[current_idx]);
    f.render_widget(deep_block, chunks[current_idx]);

    let deep_lines = if is_online {
        vec![
            Line::from(Span::styled(
                "[ ] DEEP SCAN DISABLED (Locked for Online Mailbox to optimize WAN speed)",
                Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Recursive cloud subfolder indexing disabled to prevent 0.2 msgs/sec network bottlenecks.",
                Style::default().fg(Theme::TEXT_MUTED),
            )),
            Line::from(Span::styled(
                "Standard deduplication protects all direct destination folders against duplicates.",
                Style::default().fg(Theme::SUCCESS),
            )),
            Line::from(Span::styled(
                "[P] Locked in Online Mode (prevents WAN timeouts)",
                Style::default().fg(Theme::TEXT_MUTED),
            )),
        ]
    } else {
        let deep_label = if state.deep_scan_enabled {
            Span::styled("[x] DEEP SCAN ENABLED (Recursively indexes subfolders)", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("[ ] DEEP SCAN DISABLED (Only checks direct target folder)", Style::default().fg(Theme::TEXT_MUTED))
        };
        vec![
            Line::from(deep_label),
            Line::from(Span::styled("Detects emails that users moved manually to subfolders.", Style::default().fg(Theme::TEXT_MUTED))),
            Line::from(Span::styled("More thorough and safe against scattered duplicates.", Style::default().fg(Theme::BRAND_PRIMARY))),
            Line::from(Span::styled("[P] Toggle Deep Scan", Style::default().fg(Theme::ACCENT_PRIMARY))),
        ]
    };
    f.render_widget(Paragraph::new(deep_lines), deep_inner);
    current_idx += 1;

    // 3. Resumen
    let summary = Line::from(vec![
        Span::styled("Algorithm: ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("High-speed in-memory SHA-256 hash comparison.", Style::default().fg(Theme::TEXT_MAIN)),
    ]);
    f.render_widget(Paragraph::new(summary).alignment(Alignment::Center), chunks[current_idx]);
}
