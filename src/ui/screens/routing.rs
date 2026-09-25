use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::{
    app::{AppState, RoutingGranularity, RoutingModal},
    ui::theme::Theme,
};

pub const MONTH_NAMES: [&str; 12] = [
    "01 - Enero",
    "02 - Febrero",
    "03 - Marzo",
    "04 - Abril",
    "05 - Mayo",
    "06 - Junio",
    "07 - Julio",
    "08 - Agosto",
    "09 - Setiembre",
    "10 - Octubre",
    "11 - Noviembre",
    "12 - Diciembre",
];

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // 1. Tres tarjetas interactivas de criterio
            Constraint::Length(3), // 2. Barra de Alcance y Filtro de Fecha
            Constraint::Min(7),    // 3. Vista previa del enrutamiento MAPI
            Constraint::Length(2), // 4. Barra de atajos
        ])
        .split(area);

    // =========================================================================
    // 1. TRES TARJETAS INTERACTIVAS DE CRITERIO
    // =========================================================================
    let card_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(chunks[0]);

    let is_orig_selected = state.routing_granularity == RoutingGranularity::Mirror;
    let is_years_selected = state.routing_granularity == RoutingGranularity::Years;
    let is_months_selected = state.routing_granularity == RoutingGranularity::YearsAndMonths;

    // Tarjeta 1: Conservar Estructura Original
    let card1_border_style = if is_orig_selected {
        Style::default().fg(Theme::BRAND_PRIMARY)
    } else {
        Style::default().fg(Theme::TEXT_MUTED)
    };
    let card1_border_type = if is_orig_selected {
        BorderType::Double
    } else {
        BorderType::Rounded
    };

    let card1_block = Block::default()
        .borders(Borders::ALL)
        .border_type(card1_border_type)
        .border_style(card1_border_style)
        .title(if is_orig_selected {
            " [1] Estructura Original (Nativa) "
        } else {
            " [1] Estructura Original "
        });

    let card1_inner = card1_block.inner(card_chunks[0]);
    f.render_widget(card1_block, card_chunks[0]);

    let card1_badge = if is_orig_selected {
        Span::styled(
            " [ ★ ACTIVA ] ",
            Style::default()
                .bg(Theme::SUCCESS)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " [ Tecla 1 ] ",
            Style::default().fg(Theme::TEXT_MUTED),
        )
    };

    let card1_lines = vec![
        Line::from(vec![
            Span::styled("📁 Nativa del PST", if is_orig_selected {
                Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_MAIN)
            }),
            Span::raw("  "),
            card1_badge,
        ]),
        Line::from(Span::styled(
            "Mapeo directo 1:1 a carpetas homólogas",
            if is_orig_selected {
                Style::default().fg(Theme::TEXT_MAIN)
            } else {
                Style::default().fg(Theme::TEXT_MUTED)
            },
        )),
        Line::from(Span::styled(
            "Conserva intacta la jerarquía del archivo PST",
            Style::default().fg(Theme::TEXT_MUTED),
        )),
        Line::from(Span::styled(
            "✓ Sin crear carpetas de año ni mes",
            if is_orig_selected {
                Style::default().fg(Theme::SUCCESS)
            } else {
                Style::default().fg(Theme::TEXT_MUTED)
            },
        )),
    ];
    f.render_widget(Paragraph::new(card1_lines), card1_inner);

    // Tarjeta 2: Agrupado por Años
    let card2_border_style = if is_years_selected {
        Style::default().fg(Theme::BRAND_PRIMARY)
    } else {
        Style::default().fg(Theme::TEXT_MUTED)
    };
    let card2_border_type = if is_years_selected {
        BorderType::Double
    } else {
        BorderType::Rounded
    };

    let card2_block = Block::default()
        .borders(Borders::ALL)
        .border_type(card2_border_type)
        .border_style(card2_border_style)
        .title(if is_years_selected {
            " [2] Agrupar por Años "
        } else {
            " [2] Por Años "
        });

    let card2_inner = card2_block.inner(card_chunks[1]);
    f.render_widget(card2_block, card_chunks[1]);

    let card2_badge = if is_years_selected {
        Span::styled(
            " [ ★ ACTIVA ] ",
            Style::default()
                .bg(Theme::SUCCESS)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " [ Tecla 2 ] ",
            Style::default().fg(Theme::TEXT_MUTED),
        )
    };

    let card2_lines = vec![
        Line::from(vec![
            Span::styled("📅 1 Nivel Temporal", if is_years_selected {
                Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_MAIN)
            }),
            Span::raw("  "),
            card2_badge,
        ]),
        Line::from(Span::styled(
            "Estructura: [Buzón] / <Año>",
            if is_years_selected {
                Style::default().fg(Theme::TEXT_MAIN)
            } else {
                Style::default().fg(Theme::TEXT_MUTED)
            },
        )),
        Line::from(Span::styled(
            "Clasifica los correos en carpetas anuales",
            Style::default().fg(Theme::TEXT_MUTED),
        )),
        Line::from(Span::styled(
            "✓ Ej: 2023, 2024, 2025",
            if is_years_selected {
                Style::default().fg(Theme::SUCCESS)
            } else {
                Style::default().fg(Theme::TEXT_MUTED)
            },
        )),
    ];
    f.render_widget(Paragraph::new(card2_lines), card2_inner);

    // Tarjeta 3: Agrupado por Años y Meses
    let card3_border_style = if is_months_selected {
        Style::default().fg(Theme::BRAND_PRIMARY)
    } else {
        Style::default().fg(Theme::TEXT_MUTED)
    };
    let card3_border_type = if is_months_selected {
        BorderType::Double
    } else {
        BorderType::Rounded
    };

    let card3_block = Block::default()
        .borders(Borders::ALL)
        .border_type(card3_border_type)
        .border_style(card3_border_style)
        .title(if is_months_selected {
            " [3] Por Años y Meses "
        } else {
            " [3] Años y Meses "
        });

    let card3_inner = card3_block.inner(card_chunks[2]);
    f.render_widget(card3_block, card_chunks[2]);

    let card3_badge = if is_months_selected {
        Span::styled(
            " [ ★ ACTIVA ] ",
            Style::default()
                .bg(Theme::SUCCESS)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " [ Tecla 3 ] ",
            Style::default().fg(Theme::TEXT_MUTED),
        )
    };

    let card3_lines = vec![
        Line::from(vec![
            Span::styled("📅 Jerarquía Completa", if is_months_selected {
                Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_MAIN)
            }),
            Span::raw("  "),
            card3_badge,
        ]),
        Line::from(Span::styled(
            "Estructura: [Buzón] / <Año> / <Mes>",
            if is_months_selected {
                Style::default().fg(Theme::TEXT_MAIN)
            } else {
                Style::default().fg(Theme::TEXT_MUTED)
            },
        )),
        Line::from(Span::styled(
            "Máxima granularidad y organización cronológica",
            Style::default().fg(Theme::TEXT_MUTED),
        )),
        Line::from(Span::styled(
            "✓ Ej: 2024 / 01-Enero, 2024 / 02-Febrero",
            if is_months_selected {
                Style::default().fg(Theme::SUCCESS)
            } else {
                Style::default().fg(Theme::TEXT_MUTED)
            },
        )),
    ];
    f.render_widget(Paragraph::new(card3_lines), card3_inner);

    // =========================================================================
    // 2. BARRA DE ALCANCE Y FILTRO DE FECHA (OPCIONAL)
    // =========================================================================
    let has_date_filter = state.specific_year.is_some() || state.specific_month.is_some();
    let filter_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if has_date_filter {
            Style::default().fg(Theme::WARNING)
        } else {
            Style::default().fg(Theme::ACCENT_SECONDARY)
        })
        .title(" Filtro de Fecha / Alcance Temporal (Opcional) ");

    let filter_inner = filter_block.inner(chunks[1]);
    f.render_widget(filter_block, chunks[1]);

    let filter_spans = if has_date_filter {
        let filter_desc = if let Some(y) = state.specific_year {
            if let Some(m) = state.specific_month {
                let m_name = MONTH_NAMES.get((m as usize).saturating_sub(1)).unwrap_or(&"Mes");
                format!("Año {} / {}", y, m_name)
            } else {
                format!("Año {} (Todos los meses)", y)
            }
        } else if let Some(m) = state.specific_month {
            let m_name = MONTH_NAMES.get((m as usize).saturating_sub(1)).unwrap_or(&"Mes");
            format!("Todos los años / {}", m_name)
        } else {
            "Historial completo".to_string()
        };

        vec![
            Span::styled("  🔍 Filtro Activo: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("Solo correos de: {} ", filter_desc),
                Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" • ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[F]", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" Modificar Filtro", Style::default().fg(Theme::TEXT_MAIN)),
            Span::styled("  •  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[R]", Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD)),
            Span::styled(" Quitar Filtro (Historial Completo)", Style::default().fg(Theme::TEXT_MUTED)),
        ]
    } else {
        vec![
            Span::styled("  ✓ Alcance: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                "Historial Completo (Sin filtro de fecha — Se importan todos los correos) ",
                Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" • Pulsa ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[F]", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" si deseas filtrar por un año o mes específico", Style::default().fg(Theme::TEXT_MUTED)),
        ]
    };

    f.render_widget(Paragraph::new(Line::from(filter_spans)), filter_inner);

    // =========================================================================
    // 3. VISTA PREVIA DEL ENRUTAMIENTO MAPI EN VIVO
    // =========================================================================
    let selected_mboxes = state.selected_mailboxes();
    let mbox_names = if selected_mboxes.is_empty() {
        "Buzón predeterminado".to_string()
    } else if selected_mboxes.len() == 1 {
        selected_mboxes[0].display_name.clone()
    } else {
        selected_mboxes.iter().map(|m| m.display_name.as_str()).collect::<Vec<_>>().join(" | ")
    };

    let preview_title = format!(" Vista Previa del Enrutamiento MAPI hacia: [{}] ", mbox_names);
    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(preview_title);

    let preview_inner = preview_block.inner(chunks[2]);
    f.render_widget(preview_block, chunks[2]);

    let preview_lines = match state.routing_granularity {
        RoutingGranularity::Mirror => {
            let filter_hint = if let Some(y) = state.specific_year {
                if let Some(m) = state.specific_month {
                    let m_name = MONTH_NAMES.get((m as usize).saturating_sub(1)).unwrap_or(&"Mes");
                    format!("ℹ Filtro activo: Solo correos de {} de {} serán copiados a sus carpetas nativas.", m_name, y)
                } else {
                    format!("ℹ Filtro activo: Solo correos del año {} serán copiados a sus carpetas nativas.", y)
                }
            } else if let Some(m) = state.specific_month {
                let m_name = MONTH_NAMES.get((m as usize).saturating_sub(1)).unwrap_or(&"Mes");
                format!("ℹ Filtro activo: Solo correos del mes {} serán copiados a sus carpetas nativas.", m_name)
            } else {
                "ℹ Historial completo: todos los correos se transfieren a sus carpetas homólogas.".to_string()
            };

            vec![
                Line::from(Span::styled(
                    "Conservar Estructura Original: las carpetas del PST se transfieren directamente a sus homólogas en Outlook:",
                    Style::default().fg(Theme::TEXT_MUTED),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  📁 ", Style::default().fg(Theme::BRAND_PRIMARY)),
                    Span::styled(format!("[{}]", mbox_names), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled("      ├── 📥 Bandeja de entrada /   ", Style::default().fg(Theme::ACCENT_PRIMARY)),
                    Span::styled("──▶  Mapeo directo al buzón (sin carpetas de fecha)", Style::default().fg(Theme::TEXT_MUTED)),
                ]),
                Line::from(vec![
                    Span::styled("      ├── 📤 Elementos enviados /   ", Style::default().fg(Theme::ACCENT_PRIMARY)),
                    Span::styled("──▶  Mapeo directo al buzón (sin carpetas de fecha)", Style::default().fg(Theme::TEXT_MUTED)),
                ]),
                Line::from(vec![
                    Span::styled("      └── 📂 Carpetas del PST /     ", Style::default().fg(Theme::ACCENT_PRIMARY)),
                    Span::styled("──▶  Conserva exactamente su nombre y contenido", Style::default().fg(Theme::TEXT_MUTED)),
                ]),
                Line::from(""),
                Line::from(Span::styled(filter_hint, Style::default().fg(Theme::SUCCESS))),
            ]
        }
        RoutingGranularity::Years => {
            if let Some(y) = state.specific_year {
                vec![
                    Line::from(Span::styled(
                        format!("Mapeo de correos exclusivo para el año {} hacia el buzón destino:", y),
                        Style::default().fg(Theme::TEXT_MUTED),
                    )),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("  📁 ", Style::default().fg(Theme::BRAND_PRIMARY)),
                        Span::styled(format!("[{}]", mbox_names), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(vec![
                        Span::styled(format!("      └── 📅 {} /               ──▶  ", y), Style::default().fg(Theme::ACCENT_PRIMARY)),
                        Span::styled(format!("[{}] / {}", mbox_names, y), Style::default().fg(Theme::TEXT_MAIN)),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("✓ Únicamente los correos del año {} serán transferidos a la carpeta anual dedicada.", y),
                        Style::default().fg(Theme::SUCCESS),
                    )),
                ]
            } else {
                vec![
                    Line::from(Span::styled(
                        "Clasificación automática de correos por año hacia el buzón destino:",
                        Style::default().fg(Theme::TEXT_MUTED),
                    )),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("  📁 ", Style::default().fg(Theme::BRAND_PRIMARY)),
                        Span::styled(format!("[{}]", mbox_names), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(vec![
                        Span::styled("      ├── 📅 2023 /               ──▶  ", Style::default().fg(Theme::ACCENT_PRIMARY)),
                        Span::styled(format!("[{}] / 2023", mbox_names), Style::default().fg(Theme::TEXT_MAIN)),
                    ]),
                    Line::from(vec![
                        Span::styled("      ├── 📅 2024 /               ──▶  ", Style::default().fg(Theme::ACCENT_PRIMARY)),
                        Span::styled(format!("[{}] / 2024", mbox_names), Style::default().fg(Theme::TEXT_MAIN)),
                    ]),
                    Line::from(vec![
                        Span::styled("      └── 📅 2025 /               ──▶  ", Style::default().fg(Theme::ACCENT_PRIMARY)),
                        Span::styled(format!("[{}] / 2025", mbox_names), Style::default().fg(Theme::TEXT_MAIN)),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        "✓ Cada correo se organizará en la carpeta anual correspondiente según su fecha de recepción.",
                        Style::default().fg(Theme::SUCCESS),
                    )),
                ]
            }
        }
        RoutingGranularity::YearsAndMonths => {
            if let Some(y) = state.specific_year {
                if let Some(m) = state.specific_month {
                    let m_name = MONTH_NAMES.get((m as usize).saturating_sub(1)).unwrap_or(&"Mes");
                    vec![
                        Line::from(Span::styled(
                            format!("Mapeo específico para {} / {}:", y, m_name),
                            Style::default().fg(Theme::TEXT_MUTED),
                        )),
                        Line::from(""),
                        Line::from(vec![
                            Span::styled("  📁 ", Style::default().fg(Theme::BRAND_PRIMARY)),
                            Span::styled(format!("[{}]", mbox_names), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                        ]),
                        Line::from(vec![
                            Span::styled(format!("      └── 📅 {} /", y), Style::default().fg(Theme::ACCENT_PRIMARY)),
                        ]),
                        Line::from(vec![
                            Span::styled(format!("          └── 📂 {} /   ──▶  ", m_name), Style::default().fg(Theme::ACCENT_SECONDARY)),
                            Span::styled(format!("[{}] / {} / {}", mbox_names, y, m_name), Style::default().fg(Theme::TEXT_MAIN)),
                        ]),
                        Line::from(""),
                        Line::from(Span::styled(
                            format!("✓ Únicamente se transferirán correos de {} de {}.", m_name, y),
                            Style::default().fg(Theme::SUCCESS),
                        )),
                    ]
                } else {
                    vec![
                        Line::from(Span::styled(
                            format!("Mapeo de todos los meses del año {} hacia el buzón destino:", y),
                            Style::default().fg(Theme::TEXT_MUTED),
                        )),
                        Line::from(""),
                        Line::from(vec![
                            Span::styled("  📁 ", Style::default().fg(Theme::BRAND_PRIMARY)),
                            Span::styled(format!("[{}]", mbox_names), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                        ]),
                        Line::from(vec![
                            Span::styled(format!("      └── 📅 {} /", y), Style::default().fg(Theme::ACCENT_PRIMARY)),
                        ]),
                        Line::from(vec![
                            Span::styled("          ├── 📂 01-Enero /       ──▶  ", Style::default().fg(Theme::ACCENT_SECONDARY)),
                            Span::styled(format!("[{}] / {} / 01-Enero", mbox_names, y), Style::default().fg(Theme::TEXT_MAIN)),
                        ]),
                        Line::from(vec![
                            Span::styled("          ├── 📂 02-Febrero /     ──▶  ", Style::default().fg(Theme::ACCENT_SECONDARY)),
                            Span::styled(format!("[{}] / {} / 02-Febrero", mbox_names, y), Style::default().fg(Theme::TEXT_MAIN)),
                        ]),
                        Line::from(vec![
                            Span::styled("          └── 📂 ... /            ──▶  ", Style::default().fg(Theme::ACCENT_SECONDARY)),
                            Span::styled(format!("[{}] / {} / ...", mbox_names, y), Style::default().fg(Theme::TEXT_MAIN)),
                        ]),
                        Line::from(""),
                        Line::from(Span::styled(
                            format!("✓ Todos los meses del año {} se importarán en subcarpetas mensuales dedicadas.", y),
                            Style::default().fg(Theme::SUCCESS),
                        )),
                    ]
                }
            } else {
                vec![
                    Line::from(Span::styled(
                        "Organización jerárquica completa en 2 niveles (Año / Mes) hacia el buzón destino:",
                        Style::default().fg(Theme::TEXT_MUTED),
                    )),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("  📁 ", Style::default().fg(Theme::BRAND_PRIMARY)),
                        Span::styled(format!("[{}]", mbox_names), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(vec![
                        Span::styled("      ├── 📅 2023 / ...", Style::default().fg(Theme::ACCENT_PRIMARY)),
                    ]),
                    Line::from(vec![
                        Span::styled("      └── 📅 2024 /", Style::default().fg(Theme::ACCENT_PRIMARY)),
                    ]),
                    Line::from(vec![
                        Span::styled("          ├── 📂 01-Enero /       ──▶  ", Style::default().fg(Theme::ACCENT_SECONDARY)),
                        Span::styled(format!("[{}] / 2024 / 01-Enero", mbox_names), Style::default().fg(Theme::TEXT_MAIN)),
                    ]),
                    Line::from(vec![
                        Span::styled("          ├── 📂 02-Febrero /     ──▶  ", Style::default().fg(Theme::ACCENT_SECONDARY)),
                        Span::styled(format!("[{}] / 2024 / 02-Febrero", mbox_names), Style::default().fg(Theme::TEXT_MAIN)),
                    ]),
                    Line::from(vec![
                        Span::styled("          └── 📂 ... /            ──▶  ", Style::default().fg(Theme::ACCENT_SECONDARY)),
                        Span::styled(format!("[{}] / 2024 / ...", mbox_names), Style::default().fg(Theme::TEXT_MAIN)),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        "✓ Historial completo organizado en estructura jerárquica cronológica (Año / Mes).",
                        Style::default().fg(Theme::SUCCESS),
                    )),
                ]
            }
        }
    };
    f.render_widget(Paragraph::new(preview_lines), preview_inner);

    // =========================================================================
    // 4. BARRA DE ATAJOS Y NAVEGACIÓN
    // =========================================================================
    let mut help_spans = vec![
        Span::styled("[1/2/3] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Elegir Criterio   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[←/→] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Alternar   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[F] ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
        Span::styled("Filtro de Fecha   ", Style::default().fg(Theme::TEXT_MUTED)),
    ];

    if has_date_filter {
        help_spans.push(Span::styled("[R] ", Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD)));
        help_spans.push(Span::styled("Restablecer   ", Style::default().fg(Theme::TEXT_MUTED)));
    }

    help_spans.extend(vec![
        Span::styled("[Enter] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        Span::styled("Continuar a Deduplicación   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[Esc] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Atrás", Style::default().fg(Theme::TEXT_MUTED)),
    ]);

    f.render_widget(Paragraph::new(Line::from(help_spans)).alignment(Alignment::Center), chunks[3]);

    // =========================================================================
    // 5. VENTANA MODAL DE FILTRO DE FECHA (Si está activa)
    // =========================================================================
    if state.active_routing_modal != RoutingModal::None {
        render_routing_modal(f, area, state);
    }
}

fn render_routing_modal(f: &mut Frame, area: Rect, state: &AppState) {
    let modal_width = 76.min(area.width.saturating_sub(4));
    let modal_height = 14.min(area.height.saturating_sub(2));

    let x = area.x + (area.width.saturating_sub(modal_width)) / 2;
    let y = area.y + (area.height.saturating_sub(modal_height)) / 2;
    let modal_rect = Rect::new(x, y, modal_width, modal_height);

    f.render_widget(Clear, modal_rect);

    let (title, subtitle, hints) = match state.active_routing_modal {
        RoutingModal::Criterion => (
            "--- Criterio de Enrutamiento ---".to_string(),
            "Selecciona cómo deseas estructurar las carpetas en los buzones de destino:".to_string(),
            "1/2/3 o ↑/↓ elegir | Enter confirmar | Esc volver".to_string(),
        ),
        RoutingModal::YearScope => (
            "--- Filtro de Fecha: Alcance de Años ---".to_string(),
            "Selecciona el alcance de años a procesar para el archivo PST:".to_string(),
            "↑/↓ mover | Enter confirmar | Esc volver".to_string(),
        ),
        RoutingModal::SpecificYear => (
            "--- Filtro de Fecha: Año Específico ---".to_string(),
            "Ingresa el año a procesar (ejemplo: 2024):".to_string(),
            "0-9 escribir | Backspace borrar | Enter confirmar | Esc volver".to_string(),
        ),
        RoutingModal::MonthScope => {
            let sub = if let Some(y) = state.specific_year {
                format!("Selecciona el alcance de meses para el año {}:", y)
            } else {
                "Selecciona el alcance de meses para todos los años:".to_string()
            };
            (
                "--- Filtro de Fecha: Alcance de Meses ---".to_string(),
                sub,
                "↑/↓ mover | Enter confirmar | Esc volver".to_string(),
            )
        }
        RoutingModal::SpecificMonth => {
            let sub = if let Some(y) = state.specific_year {
                format!("Seleccione el mes a procesar para el año {}:", y)
            } else {
                "Seleccione el mes a procesar:".to_string()
            };
            (
                "--- Filtro de Fecha: Mes Específico ---".to_string(),
                sub,
                "←/→ cambiar mes | Enter confirmar | Esc volver".to_string(),
            )
        }
        RoutingModal::None => return,
    };

    let modal_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Theme::BRAND_PRIMARY))
        .title(format!(" {} ", title));

    let inner = modal_block.inner(modal_rect);
    f.render_widget(modal_block, modal_rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Subtítulo e instrucciones
            Constraint::Length(1), // Espacio
            Constraint::Length(4), // Opciones interactivas
            Constraint::Min(2),    // Atajos
        ])
        .split(inner);

    let header_p = Paragraph::new(vec![
        Line::from(Span::styled(subtitle, Style::default().fg(Theme::TEXT_MUTED))),
        Line::from(Span::styled(hints, Style::default().fg(Theme::ACCENT_PRIMARY))),
    ]);
    f.render_widget(header_p, chunks[0]);

    match state.active_routing_modal {
        RoutingModal::Criterion => {
            let opt_mirror = "  1. Conservar Estructura Original (Nativa del PST - Sin agrupación temporal)";
            let opt_years = "  2. Agrupado por Años (1 nivel: [Buzón] / <Año>)";
            let opt_months = "  3. Agrupado por Años y Meses (Jerárquico: [Buzón] / <Año> / <Mes>)";

            let line_mirror = if state.routing_modal_criterion_idx == 0 {
                Line::from(Span::styled(
                    opt_mirror,
                    Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(opt_mirror, Style::default().fg(Color::Gray)))
            };

            let line_years = if state.routing_modal_criterion_idx == 1 {
                Line::from(Span::styled(
                    opt_years,
                    Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(opt_years, Style::default().fg(Color::Gray)))
            };

            let line_months = if state.routing_modal_criterion_idx == 2 {
                Line::from(Span::styled(
                    opt_months,
                    Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(opt_months, Style::default().fg(Color::Gray)))
            };

            let opts_p = Paragraph::new(vec![
                line_mirror,
                line_years,
                line_months,
            ]);
            f.render_widget(opts_p, chunks[2]);
        }
        RoutingModal::YearScope => {
            let opt_all = "  Todos los años (Historial completo por defecto)";
            let opt_spec = "  Un año específico";

            let line_all = if state.routing_modal_year_scope_idx == 0 {
                Line::from(Span::styled(
                    opt_all,
                    Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(opt_all, Style::default().fg(Color::Gray)))
            };

            let line_spec = if state.routing_modal_year_scope_idx == 1 {
                Line::from(Span::styled(
                    opt_spec,
                    Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(opt_spec, Style::default().fg(Color::Gray)))
            };

            let opts_p = Paragraph::new(vec![
                line_all,
                Line::from(""),
                line_spec,
            ]);
            f.render_widget(opts_p, chunks[2]);
        }
        RoutingModal::SpecificYear => {
            let input_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
                .title(" Año a procesar (4 dígitos) ");

            let year_text = format!("  {} █", state.routing_input_year);
            let input_p = Paragraph::new(Line::from(Span::styled(
                year_text,
                Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD),
            )))
            .block(input_block);
            f.render_widget(input_p, chunks[2]);
        }
        RoutingModal::MonthScope => {
            let (opt_all, opt_spec) = if let Some(y) = state.specific_year {
                (
                    format!("  Todos los meses del año {} (Por defecto)", y),
                    format!("  Un mes específico del año {}", y),
                )
            } else {
                (
                    "  Todos los meses de todos los años (Por defecto)".to_string(),
                    "  Un mes específico (en todos los años)".to_string(),
                )
            };

            let line_all = if state.routing_modal_month_scope_idx == 0 {
                Line::from(Span::styled(
                    opt_all,
                    Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(opt_all, Style::default().fg(Color::Gray)))
            };

            let line_spec = if state.routing_modal_month_scope_idx == 1 {
                Line::from(Span::styled(
                    opt_spec,
                    Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(opt_spec, Style::default().fg(Color::Gray)))
            };

            let opts_p = Paragraph::new(vec![
                line_all,
                Line::from(""),
                line_spec,
            ]);
            f.render_widget(opts_p, chunks[2]);
        }
        RoutingModal::SpecificMonth => {
            let month_name = MONTH_NAMES.get((state.routing_input_month as usize).saturating_sub(1)).unwrap_or(&"01 - Enero");
            let year_label = if let Some(y) = state.specific_year {
                format!("{}", y)
            } else {
                "Todos los años".to_string()
            };
            let sel_text = format!("  Año: {}  |  Mes seleccionado: [◀ {} ▶]", year_label, month_name);
            let month_p = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    sel_text,
                    Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD),
                )),
            ]);
            f.render_widget(month_p, chunks[2]);
        }
        RoutingModal::None => {}
    }
}
