use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{AppState, RoutingGranularity, TransferMode},
    ui::theme::Theme,
};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(12),   // Tarjeta Resumen Consolidado
            Constraint::Length(4), // Botones de Confirmación
        ])
        .split(area);

    // 1. Resumen Consolidado
    let summary_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Resumen de Configuración Pre-Vuelo ");

    let summary_inner = summary_block.inner(chunks[0]);
    f.render_widget(summary_block, chunks[0]);

    let selected_psts_count = state.discovered_psts.iter().filter(|p| p.selected).count();
    let total_size_mb: f64 = state.discovered_psts.iter().filter(|p| p.selected).map(|p| p.size_mb).sum();

    let mode_str = match state.transfer_mode {
        TransferMode::Copy => "COPIAR (PST intacto)",
        TransferMode::Move => "MOVER (Borrar de PST tras éxito)",
    };
    let mode_color = match state.transfer_mode {
        TransferMode::Copy => Theme::SUCCESS,
        TransferMode::Move => Theme::DANGER,
    };

    let routing_str = if state.routing_enabled {
        let scope_str = if let Some(year) = state.specific_year {
            if let Some(m) = state.specific_month {
                format!("Año {} / Mes {:02}", year, m)
            } else {
                format!("Año {} (Todos los meses)", year)
            }
        } else if let Some(m) = state.specific_month {
            format!("Todos los años / Mes {:02}", m)
        } else {
            "Todos los periodos (Historial completo)".to_string()
        };

        match state.routing_granularity {
            RoutingGranularity::Years => format!("Por Años ({})", scope_str),
            RoutingGranularity::YearsAndMonths => format!("Por Meses ({})", scope_str),
        }
    } else {
        "Desactivado (Raíz directa)".to_string()
    };

    let selected_mailboxes = state.selected_mailboxes();
    let mailbox_summary_str = if selected_mailboxes.is_empty() {
        "Ninguno seleccionado (⚠️ Se requiere al menos uno)".to_string()
    } else if selected_mailboxes.len() == 1 {
        format!("{} ({})", selected_mailboxes[0].display_name, selected_mailboxes[0].store_type)
    } else {
        let names = selected_mailboxes.iter().map(|m| m.display_name.as_str()).collect::<Vec<_>>().join(", ");
        format!("{} buzones seleccionados: {}", selected_mailboxes.len(), names)
    };

    let summary_lines = vec![
        Line::from(vec![
            Span::styled("• Perfil Outlook:    ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(if state.use_default_profile { "Predeterminado de Windows" } else { &state.custom_profile_name }, Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("• PSTs Seleccionados:", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!(" {} archivos ({:.1} MB / {:.2} GB total)", selected_psts_count, total_size_mb, total_size_mb / 1024.0), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("• Buzón(es) Destino: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(mailbox_summary_str, Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("• Modo Transferencia:", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!(" {}", mode_str), Style::default().fg(mode_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("• Enrutamiento:      ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!(" {}", routing_str), Style::default().fg(Theme::TEXT_MAIN)),
        ]),
        Line::from(vec![
            Span::styled("• Deduplicación:     ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(if state.deduplication_enabled { "Activada (Message-ID + Clave)" } else { "Desactivada" }, Style::default().fg(Theme::TEXT_MAIN)),
            Span::styled(" | Revisión Profunda: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(if state.deep_scan_enabled { "Sí (Recursiva)" } else { "No" }, Style::default().fg(Theme::TEXT_MAIN)),
        ]),
        Line::from(vec![
            Span::styled("• Throttling Adapt.: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(if state.adaptive_throttling_enabled { "Activado (Protección 429 M365)" } else { "Desactivado" }, Style::default().fg(Theme::SUCCESS)),
        ]),
    ];
    f.render_widget(Paragraph::new(summary_lines), summary_inner);

    // 2. Botones de Confirmación
    let confirm_lines = vec![
        Line::from(vec![
            Span::styled("  [ ENTER: INICIAR PROCESO DE IMPORTACIÓN ]  ", Style::default().bg(Theme::SUCCESS).fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
            Span::styled("     ", Style::default()),
            Span::styled("  [ ESC: VOLVER ATRÁS ]  ", Style::default().bg(Theme::BG_CARD).fg(Theme::TEXT_MAIN)),
        ]),
    ];
    f.render_widget(Paragraph::new(confirm_lines).alignment(Alignment::Center), chunks[1]);
}
