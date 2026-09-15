use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{AppState, RoutingGranularity},
    ui::theme::Theme,
};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // Switch de Enrutamiento
            Constraint::Length(8),  // Granularidad
            Constraint::Min(2),     // Ejemplo visual
        ])
        .split(area);

    // 1. Switch
    let switch_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Enrutamiento Inteligente ");

    let switch_inner = switch_block.inner(chunks[0]);
    f.render_widget(switch_block, chunks[0]);

    let switch_label = if state.routing_enabled {
        Span::styled("[x] ENRUTAMIENTO ACTIVADO — Clasificar correos en carpetas por fecha", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[ ] ENRUTAMIENTO DESACTIVADO — Importar directo a la raíz de la carpeta", Style::default().fg(Theme::TEXT_MUTED))
    };

    let switch_lines = vec![
        Line::from(switch_label),
        Line::from(""),
        Line::from(Span::styled("[R] Alternar Enrutamiento On/Off", Style::default().fg(Theme::ACCENT_PRIMARY))),
    ];
    f.render_widget(Paragraph::new(switch_lines), switch_inner);

    // 2. Granularidad
    let gran_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
        .title(" Estructura Jerárquica ");

    let gran_inner = gran_block.inner(chunks[1]);
    f.render_widget(gran_block, chunks[1]);

    let radio_years = if state.routing_granularity == RoutingGranularity::Years {
        Span::styled("[●] Agrupación por AÑOS (ej. Bandeja de entrada / 2024)", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[○] Agrupación por AÑOS", Style::default().fg(Theme::TEXT_MUTED))
    };

    let radio_months = if state.routing_granularity == RoutingGranularity::YearsAndMonths {
        Span::styled("[●] Agrupación por AÑOS Y MESES (ej. Bandeja de entrada / 2024 / 05-Mayo)", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[○] Agrupación por AÑOS Y MESES", Style::default().fg(Theme::TEXT_MUTED))
    };

    let gran_lines = vec![
        Line::from("Seleccione el nivel de detalle para crear subcarpetas automáticas:"),
        Line::from(""),
        Line::from(radio_years),
        Line::from(""),
        Line::from(radio_months),
        Line::from(""),
        Line::from(Span::styled("[G] Alternar Granularidad Años / Meses", Style::default().fg(Theme::ACCENT_PRIMARY))),
    ];
    f.render_widget(Paragraph::new(gran_lines), gran_inner);

    // 3. Resumen visual
    let preview_text = if state.routing_enabled {
        match state.routing_granularity {
            RoutingGranularity::Years => "📁 Vista Previa: [Bandeja de entrada] ➔ [2023] / [2024]",
            RoutingGranularity::YearsAndMonths => "📁 Vista Previa: [Bandeja de entrada] ➔ [2024] ➔ [01-Enero] / [02-Febrero]...",
        }
    } else {
        "📁 Vista Previa: [Bandeja de entrada] (Sin subcarpetas temporales)"
    };
    f.render_widget(Paragraph::new(Span::styled(preview_text, Style::default().fg(Theme::BRAND_PRIMARY))).alignment(Alignment::Center), chunks[2]);
}
