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
        .title(" Filtros Temporales (Opcional) ");

    let date_inner = date_block.inner(chunks[0]);
    f.render_widget(date_block, chunks[0]);

    let date_lines = vec![
        Line::from(vec![
            Span::styled("Rango de Importación: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("Historial Completo (Sin filtro restrictivo)", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Se importarán todos los correos contenidos en los PSTs seleccionados.", Style::default().fg(Theme::TEXT_MAIN))),
        Line::from(Span::styled("[F] Cambiar rango a año específico", Style::default().fg(Theme::TEXT_MUTED))),
    ];
    f.render_widget(Paragraph::new(date_lines), date_inner);

    // 2. Throttling Adaptativo
    let thrott_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
        .title(" Control de Rendimiento: Throttling Adaptativo ");

    let thrott_inner = thrott_block.inner(chunks[1]);
    f.render_widget(thrott_block, chunks[1]);

    let thrott_label = if state.adaptive_throttling_enabled {
        Span::styled("[x] THROTTLING ADAPTATIVO ACTIVADO (Recomendado)", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[ ] THROTTLING DESACTIVADO (Velocidad fija / Riesgo de bloqueo)", Style::default().fg(Theme::WARNING))
    };

    let thrott_lines = vec![
        Line::from(thrott_label),
        Line::from(""),
        Line::from(Span::styled("Regula automáticamente la velocidad de llamadas MAPI/COM cuando Exchange", Style::default().fg(Theme::TEXT_MUTED))),
        Line::from(Span::styled("Online o Microsoft 365 detecta saturación o emite códigos HTTP 429.", Style::default().fg(Theme::TEXT_MUTED))),
        Line::from(Span::styled("[T] Alternar Throttling Adaptativo", Style::default().fg(Theme::ACCENT_PRIMARY))),
    ];
    f.render_widget(Paragraph::new(thrott_lines), thrott_inner);

    // 3. Recomendación
    let rec = Line::from(vec![
        Span::styled("⚡ M365 Best Practice: ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
        Span::styled("Mantener el Throttling activo previene desconexiones del tenant.", Style::default().fg(Theme::TEXT_MAIN)),
    ]);
    f.render_widget(Paragraph::new(rec).alignment(Alignment::Center), chunks[2]);
}
