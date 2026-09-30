use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{AppState, RoutingGranularity, TransferMode},
    ui::theme::Theme,
};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let has_errors = state.progress.error_count > 0;
    let is_cancelled = state.progress.graceful_cancelling;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Estado superior con ribbon y 4 KPI Cards
            Constraint::Min(10),   // Dos columnas: Informes/Auditoría y Parámetros Ejecutados
            Constraint::Length(2), // Pie con atajos de salida
        ])
        .split(area);

    // =========================================================================
    // 1. ESTADO DE LA OPERACIÓN Y CARDS KPI
    // =========================================================================
    let (status_title, status_border_color, status_badge) = if is_cancelled {
        (
            " ◈ Operación Interrumpida ◈ ",
            Theme::WARNING,
            Span::styled(
                " ■ CANCELADO POR EL USUARIO ",
                Style::default()
                    .bg(Theme::WARNING)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
        )
    } else if has_errors {
        (
            " ◈ Operación Finalizada con Advertencias ◈ ",
            Theme::WARNING,
            Span::styled(
                " ▲ FINALIZADO CON INCIDENCIAS ",
                Style::default()
                    .bg(Theme::WARNING)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
        )
    } else {
        (
            " ◈ Operación Finalizada con Éxito ◈ ",
            Theme::SUCCESS,
            Span::styled(
                " ● COMPLETADO EXITOSAMENTE ",
                Style::default()
                    .bg(Theme::SUCCESS)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
        )
    };

    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(status_border_color))
        .title(status_title);

    let status_inner = status_block.inner(chunks[0]);
    f.render_widget(status_block, chunks[0]);

    // Subdividir status_inner: fila de ribbon (1 línea), espaciador (1 línea), grid de 4 KPIs (3 líneas)
    let header_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Ribbon de contexto
            Constraint::Length(1), // Espaciador sutil
            Constraint::Length(3), // Grid de 4 KPIs
        ])
        .split(status_inner);

    // Context ribbon
    let selected_mboxes = state.selected_mailboxes();
    let mbox_names = if selected_mboxes.is_empty() {
        "Buzón predeterminado".to_string()
    } else if selected_mboxes.len() == 1 {
        selected_mboxes[0].display_name.clone()
    } else {
        selected_mboxes
            .iter()
            .map(|m| m.display_name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };

    let mode_text = match state.transfer_mode {
        TransferMode::Copy => "Modo: Copiar (Intacto)",
        TransferMode::Move => "Modo: Mover (Transaccional)",
    };
    let mode_color = match state.transfer_mode {
        TransferMode::Copy => Theme::SUCCESS,
        TransferMode::Move => Theme::DANGER,
    };

    let selected_psts_count = state.discovered_psts.iter().filter(|p| p.selected).count();

    let ribbon_line = Line::from(vec![
        status_badge,
        Span::raw("  "),
        Span::styled("Destino: ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(
            mbox_names.clone(),
            Style::default()
                .fg(Theme::TEXT_MAIN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  •  ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(
            mode_text,
            Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  •  ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(
            format!("PSTs: {} archivo(s)", selected_psts_count),
            Style::default().fg(Theme::BRAND_PRIMARY),
        ),
    ]);
    f.render_widget(Paragraph::new(ribbon_line), header_chunks[0]);

    // 4 KPI Cards horizontales
    let kpi_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(header_chunks[2]);

    let total_analyzed = state.progress.imported_count + state.progress.duplicates_skipped;

    // Card 1: Importados
    let card1_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::SUCCESS))
        .title(" ✓ Importados ");
    let card1_text = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("{} ", state.progress.imported_count),
            Style::default()
                .fg(Theme::SUCCESS)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("correos", Style::default().fg(Theme::TEXT_MUTED)),
    ]))
    .alignment(Alignment::Center);
    let card1_inner = card1_block.inner(kpi_chunks[0]);
    f.render_widget(card1_block, kpi_chunks[0]);
    f.render_widget(card1_text, card1_inner);

    // Card 2: Duplicados Omitidos
    let card2_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BRAND_PRIMARY))
        .title(" ⚡ Duplicados ");
    let card2_text = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("{} ", state.progress.duplicates_skipped),
            Style::default()
                .fg(Theme::BRAND_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("omitidos", Style::default().fg(Theme::TEXT_MUTED)),
    ]))
    .alignment(Alignment::Center);
    let card2_inner = card2_block.inner(kpi_chunks[1]);
    f.render_widget(card2_block, kpi_chunks[1]);
    f.render_widget(card2_text, card2_inner);

    // Card 3: Errores
    let card3_color = if has_errors {
        Theme::DANGER
    } else {
        Theme::TEXT_MUTED
    };
    let card3_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(card3_color))
        .title(" ⚠ Errores ");
    let card3_val_color = if has_errors {
        Theme::DANGER
    } else {
        Theme::SUCCESS
    };
    let card3_text = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("{} ", state.progress.error_count),
            Style::default()
                .fg(card3_val_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("incidencias", Style::default().fg(Theme::TEXT_MUTED)),
    ]))
    .alignment(Alignment::Center);
    let card3_inner = card3_block.inner(kpi_chunks[2]);
    f.render_widget(card3_block, kpi_chunks[2]);
    f.render_widget(card3_text, card3_inner);

    // Card 4: Total Procesado
    let card4_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" 📊 Total Analizado ");
    let card4_text = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("{} ", total_analyzed),
            Style::default()
                .fg(Theme::TEXT_MAIN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("mensajes", Style::default().fg(Theme::TEXT_MUTED)),
    ]))
    .alignment(Alignment::Center);
    let card4_inner = card4_block.inner(kpi_chunks[3]);
    f.render_widget(card4_block, kpi_chunks[3]);
    f.render_widget(card4_text, card4_inner);

    // =========================================================================
    // 2. PANEL INFERIOR: INFORMES Y AUDITORÍA (54%) + PARÁMETROS EJECUTADOS (46%)
    // =========================================================================
    let is_wide = area.width >= 90;
    let (reports_area, params_area) = if is_wide {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(54), // Informes y Auditoría
                Constraint::Percentage(46), // Parámetros Ejecutados
            ])
            .split(chunks[1]);
        (cols[0], cols[1])
    } else {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(chunks[1]);
        (rows[0], rows[1])
    };

    // --- Columna Izquierda: Informes y Auditoría ---
    let reports_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" ◈ Informes y Documentación de Auditoría ◈ ");
    let reports_inner = reports_block.inner(reports_area);
    f.render_widget(reports_block, reports_area);

    let mut report_lines = Vec::new();

    // 2.1 Informe HTML
    report_lines.push(Line::from(vec![Span::styled(
        "◆ Informe Visual Interactivo HTML",
        Style::default()
            .fg(Theme::BRAND_PRIMARY)
            .add_modifier(Modifier::BOLD),
    )]));
    report_lines.push(Line::from(vec![Span::styled(
        "  Dashboard con búsqueda de correos, filtros dinámicos y desglose de carpetas.",
        Style::default().fg(Theme::TEXT_MUTED),
    )]));

    if let Some(ref path) = state.html_report_path {
        report_lines.push(Line::from(vec![
            Span::styled(
                "  ✓ Archivo: ",
                Style::default()
                    .fg(Theme::SUCCESS)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                path.display().to_string(),
                Style::default().fg(Theme::TEXT_MAIN),
            ),
        ]));
        report_lines.push(Line::from(vec![
            Span::styled(
                "  [O] ",
                Style::default()
                    .fg(Theme::BRAND_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Abrir Informe en el Navegador Web",
                Style::default()
                    .fg(Theme::TEXT_MAIN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" (Recomendado)", Style::default().fg(Theme::TEXT_MUTED)),
        ]));
        report_lines.push(Line::from(vec![
            Span::styled(
                "  [H] ",
                Style::default()
                    .fg(Theme::TEXT_MUTED)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Regenerar y volver a abrir informe interactivo",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]));
    } else {
        report_lines.push(Line::from(vec![
            Span::styled("  • Estado: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                "Listo para generar y visualizar",
                Style::default().fg(Theme::TEXT_MAIN),
            ),
        ]));
        report_lines.push(Line::from(vec![
            Span::styled(
                "  [H] ",
                Style::default()
                    .fg(Theme::SUCCESS)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Generar y Abrir Informe HTML Interactivo",
                Style::default()
                    .fg(Theme::TEXT_MAIN)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    report_lines.push(Line::from(""));

    // 2.2 Auditoría Corporativa JSON (Sin textos de borrador!)
    report_lines.push(Line::from(vec![Span::styled(
        "◆ Registro de Auditoría y Trazabilidad (JSON)",
        Style::default()
            .fg(Theme::TEXT_MAIN)
            .add_modifier(Modifier::BOLD),
    )]));
    report_lines.push(Line::from(vec![Span::styled(
        "  Trazabilidad técnica para cumplimiento normativo y control de cambios.",
        Style::default().fg(Theme::TEXT_MUTED),
    )]));

    if let Some(ref path) = state.json_audit_path {
        report_lines.push(Line::from(vec![
            Span::styled(
                "  ✓ Archivo: ",
                Style::default()
                    .fg(Theme::SUCCESS)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                path.display().to_string(),
                Style::default().fg(Theme::TEXT_MAIN),
            ),
        ]));
    } else {
        report_lines.push(Line::from(vec![Span::styled(
            "  ✓ Registro técnico archivado en el directorio de auditoría corporativa.",
            Style::default().fg(Theme::TEXT_MUTED),
        )]));
    }

    f.render_widget(Paragraph::new(report_lines), reports_inner);

    // --- Columna Derecha: Parámetros Ejecutados ---
    let params_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
        .title(" ◈ Parámetros Ejecutados ◈ ");
    let params_inner = params_block.inner(params_area);
    f.render_widget(params_block, params_area);

    let total_size_mb: f64 = state
        .discovered_psts
        .iter()
        .filter(|p| p.selected)
        .map(|p| p.size_mb)
        .sum();

    let routing_str = match state.routing_granularity {
        RoutingGranularity::Mirror => "Estructura Nativa Espejo",
        RoutingGranularity::Years => "Agrupación por Años",
        RoutingGranularity::YearsAndMonths => "Agrupación por Años y Meses",
    };

    let profile_name = if state.use_default_profile {
        "Predeterminado de Windows".to_string()
    } else {
        format!("Personalizado ({})", state.custom_profile_name)
    };

    let params_lines = vec![
        Line::from(vec![
            Span::styled(
                "• Perfil Outlook:    ",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
            Span::styled(profile_name, Style::default().fg(Theme::TEXT_MAIN)),
        ]),
        Line::from(vec![
            Span::styled(
                "• Archivos PST:      ",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
            Span::styled(
                format!("{} archivo(s) ({:.1} MB)", selected_psts_count, total_size_mb),
                Style::default().fg(Theme::BRAND_PRIMARY),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "• Buzón Destino:     ",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
            Span::styled(mbox_names, Style::default().fg(Theme::TEXT_MAIN)),
        ]),
        Line::from(vec![
            Span::styled(
                "• Transferencia:     ",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
            Span::styled(mode_text, Style::default().fg(mode_color)),
        ]),
        Line::from(vec![
            Span::styled(
                "• Estructura:        ",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
            Span::styled(routing_str, Style::default().fg(Theme::ACCENT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled(
                "• Deduplicación:     ",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
            Span::styled(
                if state.deep_scan_enabled {
                    "Revisión Profunda MAPI"
                } else {
                    "Detección Estándar"
                },
                Style::default().fg(Theme::TEXT_MAIN),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "• Anti-Throttling:   ",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
            Span::styled(
                if state.adaptive_throttling_enabled {
                    "Adaptativo Activo"
                } else {
                    "Desactivado"
                },
                Style::default().fg(if state.adaptive_throttling_enabled {
                    Theme::SUCCESS
                } else {
                    Theme::TEXT_MUTED
                }),
            ),
        ]),
    ];

    f.render_widget(Paragraph::new(params_lines), params_inner);

    // =========================================================================
    // 3. PIE DE PANTALLA: CIERRE E INSTRUCCIONES
    // =========================================================================
    let exit_line = Line::from(vec![
        Span::styled("Presiona ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(
            "[Enter]",
            Style::default()
                .fg(Theme::BRAND_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" o ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(
            "[Q]",
            Style::default()
                .fg(Theme::BRAND_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " para cerrar la aplicación y volver a la terminal.",
            Style::default().fg(Theme::TEXT_MUTED),
        ),
    ]);
    f.render_widget(
        Paragraph::new(exit_line).alignment(Alignment::Center),
        chunks[2],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{MailboxItem, PstItem};
    use ratatui::{backend::TestBackend, Terminal};
    use std::path::PathBuf;

    #[test]
    fn test_render_completion_initial_state() {
        let mut state = AppState::new();
        state.progress.imported_count = 1331;
        state.progress.duplicates_skipped = 10;
        state.progress.error_count = 0;
        state.discovered_psts = vec![PstItem {
            name: "test.pst".to_string(),
            path: r"C:\Correo\test.pst".to_string(),
            size_mb: 45.2,
            selected: true,
        }];
        state.discovered_mailboxes = vec![MailboxItem {
            display_name: "test@empresa.com".to_string(),
            store_type: "ExchangeOnline".to_string(),
            size_display: "10 MB".to_string(),
            file_path: None,
            selected: true,
        }];

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render(f, f.area(), &state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_completion_with_html_and_json_reports() {
        let mut state = AppState::new();
        state.progress.imported_count = 500;
        state.progress.duplicates_skipped = 5;
        state.progress.error_count = 2;
        state.html_report_path = Some(PathBuf::from(r"logs\2026-09-30\report_09-40-25.html"));
        state.json_audit_path = Some(PathBuf::from(r"logs\2026-09-30\run_09-40-25.json"));

        let backend = TestBackend::new(120, 35);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render(f, f.area(), &state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_completion_compact_terminal() {
        let mut state = AppState::new();
        state.progress.graceful_cancelling = true;

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render(f, f.area(), &state);
            })
            .unwrap();
    }
}
