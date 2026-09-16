use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::ui::theme::Theme;

/// Renderiza el pie de página con atajos de navegación y la marca de agua corporativa
/// fija en la esquina inferior derecha:
///   ⏳ TIMELESS
///      SUPPORT
pub fn render_footer(f: &mut Frame, area: Rect, shortcuts: &[(&str, &str)]) {
    // Bloque exterior del footer con bordes sutiles
    let footer_block = Block::default()
        .borders(Borders::TOP)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::TEXT_MUTED));

    let inner = footer_block.inner(area);
    f.render_widget(footer_block, area);

    // Dividir en horizontal: Izquierda (Atajos de teclado) y Derecha (Marca de Agua)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(20),       // Atajos
            Constraint::Length(18),     // Marca de agua Timeless Support
        ])
        .split(inner);

    // 1. Renderizar atajos
    let mut spans = Vec::new();
    for (i, (key, desc)) in shortcuts.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("   ", Style::default()));
        }
        spans.push(Span::styled(
            format!("[{}] ", key),
            Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(*desc, Style::default().fg(Theme::TEXT_MUTED)));
    }
    let shortcuts_p = Paragraph::new(Line::from(spans));
    f.render_widget(shortcuts_p, chunks[0]);

    // 2. Renderizar Marca de Agua corporativa en 2 líneas
    let watermark_lines = vec![
        Line::from(vec![
            Span::styled("◈ ", Style::default().fg(Theme::BRAND_PRIMARY)),
            Span::styled(
                "TIMELESS",
                Style::default()
                    .fg(Theme::BRAND_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "   SUPPORT",
                Style::default()
                    .fg(Theme::BRAND_SECONDARY)
                    .add_modifier(Modifier::DIM),
            ),
        ]),
    ];
    let watermark_p = Paragraph::new(watermark_lines).alignment(Alignment::Right);
    f.render_widget(watermark_p, chunks[1]);
}
