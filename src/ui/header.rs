use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::ui::theme::Theme;

/// Renderiza el encabezado superior con el título de la app y el paso actual del wizard
pub fn render_header(f: &mut Frame, area: Rect, step_title: &str, current_step: usize, total_steps: usize) {
    let header_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY));

    let inner = header_block.inner(area);
    f.render_widget(header_block, area);

    let left_text = Line::from(vec![
        Span::styled(" ◈ ", Style::default().fg(Theme::ACCENT_PRIMARY)),
        Span::styled(
            "OUTLOOK ORGANIZER TS",
            Style::default()
                .fg(Theme::TEXT_MAIN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  v1.0.0", Style::default().fg(Theme::TEXT_MUTED)),
    ]);

    let right_text = Line::from(vec![
        Span::styled(
            format!("[Paso {} de {}: {}] ", current_step, total_steps, step_title),
            Style::default()
                .fg(Theme::ACCENT_SECONDARY)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let header_p = Paragraph::new(left_text);
    f.render_widget(header_p, inner);

    let step_p = Paragraph::new(right_text).alignment(Alignment::Right);
    f.render_widget(step_p, inner);
}
