use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::{app::AppState, ui::theme::Theme};

/// Genera una celda con la barra gráfica de porcentaje de ocupación
pub fn build_storage_bar_cell(percent: f64, is_focused: bool) -> Cell<'static> {
    let clamped = percent.clamp(0.0, 100.0);
    let bar_width = 10;
    let filled_count = ((clamped / 100.0) * (bar_width as f64)).round() as usize;
    let filled_count = filled_count.min(bar_width);
    let empty_count = bar_width.saturating_sub(filled_count);

    let (bar_color, status_style) = if clamped >= 90.0 {
        (Theme::DANGER, Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD))
    } else if clamped >= 75.0 {
        (Theme::WARNING, Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD))
    } else {
        (Theme::SUCCESS, Style::default().fg(Theme::SUCCESS))
    };

    let filled_str = "█".repeat(filled_count);
    let empty_str = "░".repeat(empty_count);

    let bracket_color = if is_focused { Theme::TEXT_MAIN } else { Theme::BORDER_INACTIVE };

    let spans = vec![
        Span::styled("[", Style::default().fg(bracket_color)),
        Span::styled(filled_str, Style::default().fg(bar_color).add_modifier(Modifier::BOLD)),
        Span::styled(empty_str, Style::default().fg(Theme::BORDER_INACTIVE)),
        Span::styled("] ", Style::default().fg(bracket_color)),
        Span::styled(format!("{:>5.1}%", clamped), status_style),
    ];

    Cell::from(Line::from(spans))
}

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let has_warning = state.mailbox_warning_notice.is_some();
    let show_detail_panel = area.height >= 26;

    let constraints = match (show_detail_panel, has_warning) {
        (true, true) => vec![
            Constraint::Length(5), // Resumen de perfil y buzones
            Constraint::Min(8),    // Tabla interactiva con barras
            Constraint::Length(6), // Panel de Diagnóstico del Buzón Enfocado
            Constraint::Length(2), // Aviso de advertencia
            Constraint::Length(2), // Atajos
        ],
        (true, false) => vec![
            Constraint::Length(5), // Resumen de perfil y buzones
            Constraint::Min(8),    // Tabla interactiva con barras
            Constraint::Length(6), // Panel de Diagnóstico del Buzón Enfocado
            Constraint::Length(2), // Atajos
        ],
        (false, true) => vec![
            Constraint::Length(5), // Resumen de perfil y buzones
            Constraint::Min(8),    // Tabla interactiva con barras
            Constraint::Length(2), // Aviso de advertencia
            Constraint::Length(2), // Atajos
        ],
        (false, false) => vec![
            Constraint::Length(5), // Resumen de perfil y buzones
            Constraint::Min(8),    // Tabla interactiva con barras
            Constraint::Length(2), // Atajos
        ],
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    // 1. Resumen de Perfil MAPI y Capacidad Global
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Detección de Buzones y Capacidad MAPI ");

    let header_inner = header_block.inner(chunks[0]);
    f.render_widget(header_block, chunks[0]);

    let profile_desc = if state.use_default_profile {
        "Perfil predeterminado del sistema".to_string()
    } else if state.custom_profile_name.trim().is_empty() {
        "Perfil manual (no especificado)".to_string()
    } else {
        format!("Perfil manual: {}", state.custom_profile_name)
    };

    let selected_mailboxes = state.selected_mailboxes();
    let selected_count = selected_mailboxes.len();
    let total_count = state.discovered_mailboxes.len();

    let combined_free_gb: f64 = selected_mailboxes.iter().map(|m| m.get_free_gb()).sum();
    let combined_total_gb: f64 = selected_mailboxes.iter().map(|m| m.get_total_gb()).sum();

    let mut header_line2 = vec![
        Span::styled("Buzones detectados: ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(format!("{}", total_count), Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled(" | Marcados como destino: ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(
            format!("{}", selected_count),
            Style::default().fg(if selected_count > 0 { Theme::SUCCESS } else { Theme::WARNING }).add_modifier(Modifier::BOLD),
        ),
    ];

    if selected_count > 0 {
        header_line2.push(Span::styled(" | Capacidad libre en seleccionados: ", Style::default().fg(Theme::TEXT_MUTED)));
        header_line2.push(Span::styled(
            format!("{:.1} GB / {:.0} GB", combined_free_gb, combined_total_gb),
            Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD),
        ));
    } else {
        header_line2.push(Span::styled(" (Marca con [Espacio] al menos un buzón)", Style::default().fg(Theme::TEXT_MUTED)));
    }

    let header_lines = vec![
        Line::from(vec![
            Span::styled("Perfil MAPI en uso: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(profile_desc, Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            if state.is_loading_mailboxes {
                Span::styled("   [⏳ Consultando Outlook y cuotas...]", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD))
            } else {
                Span::styled("   [✓ Conexión establecida]", Style::default().fg(Theme::SUCCESS))
            },
        ]),
        Line::from(header_line2),
    ];
    f.render_widget(Paragraph::new(header_lines), header_inner);

    // 2. Tabla Interactiva de Buzones con Barra de Ocupación
    if state.is_loading_mailboxes && state.discovered_mailboxes.is_empty() {
        let loading_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
            .title(" Buzones de Destino ");
        let loading_p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("⏳ Conectando con Microsoft Outlook y calculando cuotas...", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("Por favor espere un momento mientras se leen los almacenes MAPI y niveles de ocupación.", Style::default().fg(Theme::TEXT_MUTED))),
        ])
        .alignment(Alignment::Center)
        .block(loading_block);
        f.render_widget(loading_p, chunks[1]);
    } else {
        let header_cells = [
            "Sel",
            "Buzón / Almacén",
            "Tipo Almacén",
            "Uso / Cuota",
            "Ocupación (Barra)",
            "Salud",
        ]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)));
        let header = Row::new(header_cells).height(1).bottom_margin(1);

        let rows = state.discovered_mailboxes.iter().enumerate().map(|(idx, item)| {
            let is_focused = idx == state.selected_mailbox_idx;
            let checkbox = if item.selected { "[x]" } else { "[ ]" };
            let sel_cell = Cell::from(checkbox).style(if item.selected {
                Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_MUTED)
            });

            let name_prefix = if is_focused { "▶ " } else { "  " };
            let name_cell = Cell::from(format!("{}{}", name_prefix, item.display_name));

            let type_cell = Cell::from(format!("({})", item.store_type)).style(
                if item.store_type == "ExchangeOnline" {
                    Style::default().fg(Theme::BRAND_PRIMARY)
                } else if item.store_type == "SharedMailbox" {
                    Style::default().fg(Theme::ACCENT_PRIMARY)
                } else if item.store_type == "Delegate" {
                    Style::default().fg(Theme::ACCENT_SECONDARY)
                } else if item.store_type == "PST" {
                    Style::default().fg(Theme::WARNING)
                } else {
                    Style::default().fg(Theme::TEXT_MUTED)
                }
            );

            let usage_ratio_str = format!("{:.1} GB / {}", item.get_used_gb(), item.get_quota_display());
            let usage_cell = Cell::from(usage_ratio_str).style(Style::default().fg(Theme::TEXT_MAIN));

            let pct = item.get_usage_percent();
            let bar_cell = build_storage_bar_cell(pct, is_focused);

            let (health_label, health_color) = item.health_status();
            let health_symbol = if pct >= 90.0 {
                "✖ "
            } else if pct >= 75.0 {
                "▲ "
            } else {
                "● "
            };
            let health_cell = Cell::from(format!("{}{}", health_symbol, health_label))
                .style(Style::default().fg(health_color).add_modifier(if pct >= 75.0 { Modifier::BOLD } else { Modifier::empty() }));

            let row = Row::new(vec![sel_cell, name_cell, type_cell, usage_cell, bar_cell, health_cell]);
            if is_focused {
                row.style(Style::default().bg(Theme::BG_CARD).fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD))
            } else {
                row
            }
        });

        let table = Table::new(
            rows,
            [
                Constraint::Length(5),  // Sel
                Constraint::Min(24),    // Buzón / Almacén
                Constraint::Length(16), // Tipo
                Constraint::Length(18), // Uso / Cuota
                Constraint::Length(23), // Barra Ocupación
                Constraint::Length(12), // Salud
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
                .title(" Buzones de Destino Disponibles (Espacio: Marcar/Desmarcar) "),
        );

        f.render_widget(table, chunks[1]);
    }

    // 3. Panel de Diagnóstico y Proyección de Capacidad del Buzón Enfocado
    let mut next_chunk_idx = 2;
    if show_detail_panel {
        let focused_item = state.discovered_mailboxes.get(state.selected_mailbox_idx);
        let detail_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::BORDER_INACTIVE))
            .title(" Diagnóstico de Capacidad y Proyección de Transferencia ");
        let detail_inner = detail_block.inner(chunks[2]);
        f.render_widget(detail_block, chunks[2]);

        let detail_lines = if let Some(item) = focused_item {
            let (health_label, health_color) = item.health_status();
            let free_gb = item.get_free_gb();
            let used_gb = item.get_used_gb();
            let total_gb = item.get_total_gb();
            let pct = item.get_usage_percent();

            let line1 = Line::from(vec![
                Span::styled("Buzón enfocado: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(&item.display_name, Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" ({})   ", item.store_type), Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled("Estado: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(format!("{} ({:.1}%)   ", health_label, pct), Style::default().fg(health_color).add_modifier(Modifier::BOLD)),
                Span::styled("Uso: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(format!("{:.2} GB / {:.0} GB   ", used_gb, total_gb), Style::default().fg(Theme::TEXT_MAIN)),
                Span::styled("Libre: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(format!("{:.2} GB", free_gb), Style::default().fg(health_color).add_modifier(Modifier::BOLD)),
            ]);

            let path_desc = item.file_path.as_deref().unwrap_or("Modo Directo en la Nube (Sin archivo local .ost)");
            let line2 = Line::from(vec![
                Span::styled("Ubicación de datos: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(path_desc, Style::default().fg(Theme::TEXT_MAIN)),
            ]);

            // Proyección de PSTs seleccionados
            let total_pst_mb = state.total_selected_psts_size_mb();
            let total_pst_gb = total_pst_mb / 1024.0;
            let line3 = if total_pst_mb > 0.0 {
                if total_pst_gb > free_gb {
                    Line::from(vec![
                        Span::styled("⚠️ ALERTA DE CUOTA: ", Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD)),
                        Span::styled(
                            format!(
                                "Los PSTs seleccionados ({:.2} GB) exceden el espacio libre restante ({:.2} GB). Riesgo de rebose del buzón.",
                                total_pst_gb, free_gb
                            ),
                            Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD),
                        ),
                    ])
                } else if total_pst_gb > free_gb * 0.85 {
                    Line::from(vec![
                        Span::styled("▲ PRECAUCIÓN: ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
                        Span::styled(
                            format!(
                                "Los PSTs ({:.2} GB) ocuparán casi todo el espacio libre ({:.2} GB restantes tras importación).",
                                total_pst_gb,
                                (free_gb - total_pst_gb).max(0.0)
                            ),
                            Style::default().fg(Theme::WARNING),
                        ),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled("✓ PROYECCIÓN FAVORABLE: ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
                        Span::styled(
                            format!(
                                "Capacidad suficiente. Tras transferir {:.2} GB de PSTs, quedarán {:.2} GB libres en este buzón.",
                                total_pst_gb,
                                free_gb - total_pst_gb
                            ),
                            Style::default().fg(Theme::TEXT_MUTED),
                        ),
                    ])
                }
            } else {
                Line::from(vec![
                    Span::styled("ℹ Proyección: ", Style::default().fg(Theme::ACCENT_PRIMARY)),
                    Span::styled(
                        format!(
                            "No hay PSTs seleccionados en el paso anterior. Capacidad neta disponible para ingesta: {:.2} GB.",
                            free_gb
                        ),
                        Style::default().fg(Theme::TEXT_MUTED),
                    ),
                ])
            };

            vec![line1, line2, line3]
        } else {
            vec![Line::from(Span::styled("No hay buzones disponibles para diagnosticar.", Style::default().fg(Theme::TEXT_MUTED)))]
        };

        f.render_widget(Paragraph::new(detail_lines), detail_inner);
        next_chunk_idx += 1;
    }

    // 4. Advertencia si aplica
    if has_warning {
        if let Some(ref notice) = state.mailbox_warning_notice {
            let warn_p = Paragraph::new(Line::from(vec![
                Span::styled("⚠️  ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
                Span::styled(notice.as_str(), Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
            ]))
            .alignment(Alignment::Center);
            f.render_widget(warn_p, chunks[next_chunk_idx]);
        }
        next_chunk_idx += 1;
    }

    // 5. Atajos de teclado
    let help_line = Line::from(vec![
        Span::styled("[↑/↓] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Navegar   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[Espacio] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Marcar/Desmarcar   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[A] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Todos   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[N] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Ninguno   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[R] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Recargar de Outlook   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[Enter] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        Span::styled("Continuar", Style::default().fg(Theme::TEXT_MUTED)),
    ]);
    f.render_widget(Paragraph::new(help_line).alignment(Alignment::Center), chunks[next_chunk_idx]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppState, MailboxItem, PstItem};
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_render_mailbox_screen_standard_resolution() {
        let mut state = AppState::new();
        state.discovered_mailboxes = vec![
            MailboxItem {
                display_name: "personal@empresa.com".to_string(),
                store_type: "ExchangeOnline".to_string(),
                size_display: "15.00 GB".to_string(),
                file_path: Some(r"C:\Users\test\personal.ost".to_string()),
                selected: true,
                used_bytes: Some(16_106_127_360),
                total_bytes: Some(53_687_091_200),
                quota_display: Some("50 GB".to_string()),
                usage_percent: Some(30.0),
            },
            MailboxItem {
                display_name: "ventas-compartido@empresa.com".to_string(),
                store_type: "SharedMailbox".to_string(),
                size_display: "46.00 GB".to_string(),
                file_path: None,
                selected: false,
                used_bytes: Some(49_392_123_904),
                total_bytes: Some(53_687_091_200),
                quota_display: Some("50 GB".to_string()),
                usage_percent: Some(92.0),
            },
        ];
        state.discovered_psts = vec![PstItem {
            name: "archivo.pst".to_string(),
            path: r"C:\Correo\archivo.pst".to_string(),
            size_mb: 2048.0, // 2 GB
            selected: true,
        }];

        let backend = TestBackend::new(110, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render(f, f.area(), &state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_mailbox_screen_compact_resolution() {
        let state = AppState::new();
        let backend = TestBackend::new(80, 22);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render(f, f.area(), &state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_mailbox_loading_state() {
        let mut state = AppState::new();
        state.is_loading_mailboxes = true;
        state.discovered_mailboxes.clear();

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render(f, f.area(), &state);
            })
            .unwrap();
    }

    #[test]
    fn test_storage_bar_generation_bounds() {
        let _ = build_storage_bar_cell(0.0, false);
        let _ = build_storage_bar_cell(50.0, true);
        let _ = build_storage_bar_cell(85.0, false);
        let _ = build_storage_bar_cell(99.0, true);
        let _ = build_storage_bar_cell(150.0, false);
    }
}
