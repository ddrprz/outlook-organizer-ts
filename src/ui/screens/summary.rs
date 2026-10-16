use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{AppState, RoutingGranularity, TransferMode},
    ui::{format::format_size_mb, theme::Theme},
};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let has_warning = state.summary_warning_notice.is_some();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if has_warning {
            vec![
                Constraint::Min(11),   // Tarjeta Resumen Consolidado
                Constraint::Length(3), // Banner de Advertencia de Bloqueo
                Constraint::Length(4), // Botones de Confirmación
            ]
        } else {
            vec![
                Constraint::Min(13),   // Tarjeta Resumen Consolidado
                Constraint::Length(4), // Botones de Confirmación
            ]
        })
        .split(area);

    // 1. Resumen Consolidado
    let summary_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Resumen Previo de Configuración ");

    let summary_inner = summary_block.inner(chunks[0]);
    f.render_widget(summary_block, chunks[0]);

    let selected_psts_count = state.discovered_psts.iter().filter(|p| p.selected).count();
    let total_size_mb: f64 = state.discovered_psts.iter().filter(|p| p.selected).map(|p| p.size_mb).sum();

    let mode_str = match state.transfer_mode {
        TransferMode::Copy => "COPIAR (No destructivo: PST original intacto en disco)",
        TransferMode::Move => "MOVER (Transfiere correos y los elimina del PST tras verificación segura)",
    };
    let mode_color = match state.transfer_mode {
        TransferMode::Copy => Theme::SUCCESS,
        TransferMode::Move => Theme::WARNING,
    };

    let (routing_criterion_str, routing_criterion_color) = match state.routing_granularity {
        RoutingGranularity::Mirror => (
            "[1] Conservar Estructura Original (1:1 nativa del PST)",
            Theme::BRAND_PRIMARY,
        ),
        RoutingGranularity::Years => (
            "[2] Agrupar por Años (Jerarquía: [Buzón] / <Año> / <Carpetas>)",
            Theme::ACCENT_PRIMARY,
        ),
        RoutingGranularity::YearsAndMonths => (
            "[3] Agrupar por Año y Mes (Jerarquía: [Buzón] / <Año> / <Mes> / <Carpetas>)",
            Theme::SUCCESS,
        ),
    };

    let (date_filter_str, date_filter_color) = if !state.routing_all_years || !state.routing_all_months {
        let years_str = state.format_years_filter_display();
        let months_str = state.format_months_filter_display();
        (
            format!("Filtro Activo: Años [{}] | Meses [{}]", years_str, months_str),
            Theme::WARNING,
        )
    } else {
        (
            "Historial Completo (Sin filtro restrictivo: todos los años y meses)".to_string(),
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

    let is_online = state.is_target_online();
    let selected_mailboxes = state.selected_mailboxes();
    let mailbox_summary_str = if let Some(target) = selected_mailboxes.first() {
        if is_online {
            format!("{} ({} - ☁ Modo Online)", target.display_name, target.store_type)
        } else {
            format!("{} ({})", target.display_name, target.store_type)
        }
    } else {
        "Ninguno seleccionado (▲ Se requiere seleccionar un buzón destino)".to_string()
    };

    let mut summary_lines = vec![
        Line::from(vec![
            Span::styled("• Perfil Outlook MAPI: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                if state.use_default_profile { "Predeterminado de Windows" } else { &state.custom_profile_name },
                Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("• Archivos PST Origen:   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{} archivos seleccionados ({})", selected_psts_count, format_size_mb(total_size_mb)),
                Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("• Buzón Destino:          ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(mailbox_summary_str, Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("• Modo de Transferencia:  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!(" {}", mode_str), Style::default().fg(mode_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("• Enrutamiento Temporal:  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!(" {}", routing_criterion_str),
                Style::default().fg(routing_criterion_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("• Filtro de Fechas:       ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!(" {}", date_filter_str),
                Style::default().fg(date_filter_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("• Carpetas Seleccionadas: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!(" {}", folder_filter_str), Style::default().fg(Theme::TEXT_MAIN)),
        ]),
        Line::from(vec![
            Span::styled("• Deduplicación:          ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(if state.deduplication_enabled { "Activada (Message-ID + Clave compuesta)" } else { "Desactivada" }, Style::default().fg(Theme::TEXT_MAIN)),
            Span::styled(" | Revisión Profunda: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                if is_online {
                    "Desactivada (Auto-ajustada para buzón Online)"
                } else if state.deep_scan_enabled {
                    "Sí (Recursiva)"
                } else {
                    "No"
                },
                if is_online {
                    Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_MAIN)
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("• Throttling Adaptativo:  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(if state.adaptive_throttling_enabled { "Activado (Protección anti-429 M365)" } else { "Desactivado" }, Style::default().fg(Theme::SUCCESS)),
        ]),
    ];

    if is_online {
        summary_lines.push(Line::from(vec![
            Span::styled("▲ Aviso Modo Online:      ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
            Span::styled(
                "Revisión profunda auto-desactivada para prevenir lentitud WAN (deduplicación directa 100% activa).",
                Style::default().fg(Theme::WARNING),
            ),
        ]));
    }

    f.render_widget(Paragraph::new(summary_lines), summary_inner);

    let button_idx = if has_warning {
        if let Some(ref notice) = state.summary_warning_notice {
            let warn_p = Paragraph::new(Line::from(vec![
                Span::styled("▲  ", Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD)),
                Span::styled(notice.as_str(), Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD)),
            ]))
            .alignment(Alignment::Center);
            f.render_widget(warn_p, chunks[1]);
        }
        2
    } else {
        1
    };

    // 2. Botones de Confirmación
    let confirm_lines = vec![
        Line::from(vec![
            Span::styled("  [ ENTER: INICIAR PROCESO DE MIGRACIÓN ]  ", Style::default().bg(Theme::SUCCESS).fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
            Span::styled("     ", Style::default()),
            Span::styled("  [ ESC: REGRESAR A REGLAS ]  ", Style::default().bg(Theme::BG_CARD).fg(Theme::TEXT_MAIN)),
        ]),
    ];
    f.render_widget(Paragraph::new(confirm_lines).alignment(Alignment::Center), chunks[button_idx]);
}
