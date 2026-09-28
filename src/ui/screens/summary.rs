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
            Constraint::Min(13),   // Tarjeta Resumen Consolidado
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
        TransferMode::Copy => "COPIAR (No destructivo: PST original intacto en disco)",
        TransferMode::Move => "MOVER (Destructivo: correos eliminados del PST tras éxito)",
    };
    let mode_color = match state.transfer_mode {
        TransferMode::Copy => Theme::SUCCESS,
        TransferMode::Move => Theme::DANGER,
    };

    let (routing_criterion_str, routing_criterion_color) = match state.routing_granularity {
        RoutingGranularity::Mirror => (
            "[1] Conservar Estructura Original (Nativa del PST - Mapeo directo 1:1)",
            Theme::BRAND_PRIMARY,
        ),
        RoutingGranularity::Years => (
            "[2] Agrupación por Años (Jerárquico: [Buzón] / <Año> / <Carpetas>)",
            Theme::ACCENT_PRIMARY,
        ),
        RoutingGranularity::YearsAndMonths => (
            "[3] Agrupación por Años y Meses (Jerárquico: [Buzón] / <Año> / <Mes> / <Carpetas>)",
            Theme::SUCCESS,
        ),
    };

    let (date_filter_str, date_filter_color) = if let Some(year) = state.specific_year {
        if let Some(m) = state.specific_month {
            let m_name = crate::ui::screens::routing::MONTH_NAMES
                .get((m as usize).saturating_sub(1))
                .copied()
                .unwrap_or("Desconocido");
            (
                format!("Filtro Activo: Solo año {} y mes {}", year, m_name),
                Theme::WARNING,
            )
        } else {
            (
                format!("Filtro Activo: Solo año {} (Todos los meses)", year),
                Theme::WARNING,
            )
        }
    } else if let Some(m) = state.specific_month {
        let m_name = crate::ui::screens::routing::MONTH_NAMES
            .get((m as usize).saturating_sub(1))
            .copied()
            .unwrap_or("Desconocido");
        (
            format!("Filtro Activo: Solo mes {} (Todos los años)", m_name),
            Theme::WARNING,
        )
    } else {
        (
            "Historial Completo (Sin filtro de fecha: todos los años y meses)".to_string(),
            Theme::TEXT_MAIN,
        )
    };

    let selected_folders = state.folder_tree.selected_paths();
    let folder_filter_str = if selected_folders.is_empty() {
        "Todas las carpetas estándar del PST".to_string()
    } else {
        let count = selected_folders.len();
        if count <= 3 {
            format!("{} carpetas seleccionadas ({})", count, selected_folders.join(", "))
        } else {
            let sample = selected_folders[..2].join(", ");
            format!("{} carpetas seleccionadas (incluye {}, ...)", count, sample)
        }
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
            Span::styled("• Perfil Outlook MAPI: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                if state.use_default_profile { "Predeterminado de Windows" } else { &state.custom_profile_name },
                Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("• PSTs de Origen:      ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{} archivos seleccionados ({:.1} MB / {:.2} GB total)", selected_psts_count, total_size_mb, total_size_mb / 1024.0),
                Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("• Buzón(es) Destino:   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(mailbox_summary_str, Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("• Modo Transferencia:  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!(" {}", mode_str), Style::default().fg(mode_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("• Criterio Enrutado:   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!(" {}", routing_criterion_str),
                Style::default().fg(routing_criterion_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("• Filtro de Fechas:    ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!(" {}", date_filter_str),
                Style::default().fg(date_filter_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("• Carpetas Incluidas:  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!(" {}", folder_filter_str), Style::default().fg(Theme::TEXT_MAIN)),
        ]),
        Line::from(vec![
            Span::styled("• Deduplicación:       ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(if state.deduplication_enabled { "Activada (Message-ID + Clave)" } else { "Desactivada" }, Style::default().fg(Theme::TEXT_MAIN)),
            Span::styled(" | Revisión Profunda: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(if state.deep_scan_enabled { "Sí (Recursiva)" } else { "No" }, Style::default().fg(Theme::TEXT_MAIN)),
        ]),
        Line::from(vec![
            Span::styled("• Throttling Adapt.:   ", Style::default().fg(Theme::TEXT_MUTED)),
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
