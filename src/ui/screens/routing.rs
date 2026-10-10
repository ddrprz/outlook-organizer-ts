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
    "01 - January",
    "02 - February",
    "03 - March",
    "04 - April",
    "05 - May",
    "06 - June",
    "07 - July",
    "08 - August",
    "09 - September",
    "10 - October",
    "11 - November",
    "12 - December",
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
            " [1] Original Structure (Native) "
        } else {
            " [1] Original Structure "
        });

    let card1_inner = card1_block.inner(card_chunks[0]);
    f.render_widget(card1_block, card_chunks[0]);

    let card1_badge = if is_orig_selected {
        Span::styled(
            " [ ★ ACTIVE ] ",
            Style::default()
                .bg(Theme::SUCCESS)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " [ Key 1 ] ",
            Style::default().fg(Theme::TEXT_MUTED),
        )
    };

    let card1_lines = vec![
        Line::from(vec![
            Span::styled("◈ PST Native", if is_orig_selected {
                Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_MAIN)
            }),
            Span::raw("  "),
            card1_badge,
        ]),
        Line::from(Span::styled(
            "Direct 1:1 mapping to matching folders",
            if is_orig_selected {
                Style::default().fg(Theme::TEXT_MAIN)
            } else {
                Style::default().fg(Theme::TEXT_MUTED)
            },
        )),
        Line::from(Span::styled(
            "Preserves PST file hierarchy intact",
            Style::default().fg(Theme::TEXT_MUTED),
        )),
        Line::from(Span::styled(
            "✓ Without creating year or month folders",
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
            " [2] Group by Year "
        } else {
            " [2] By Year "
        });

    let card2_inner = card2_block.inner(card_chunks[1]);
    f.render_widget(card2_block, card_chunks[1]);

    let card2_badge = if is_years_selected {
        Span::styled(
            " [ ★ ACTIVE ] ",
            Style::default()
                .bg(Theme::SUCCESS)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " [ Key 2 ] ",
            Style::default().fg(Theme::TEXT_MUTED),
        )
    };

    let card2_lines = vec![
        Line::from(vec![
            Span::styled("◷ 1 Time Level", if is_years_selected {
                Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_MAIN)
            }),
            Span::raw("  "),
            card2_badge,
        ]),
        Line::from(Span::styled(
            "Structure: [Mailbox] / <Year>",
            if is_years_selected {
                Style::default().fg(Theme::TEXT_MAIN)
            } else {
                Style::default().fg(Theme::TEXT_MUTED)
            },
        )),
        Line::from(Span::styled(
            "Classifies emails into annual folders",
            Style::default().fg(Theme::TEXT_MUTED),
        )),
        Line::from(Span::styled(
            "✓ E.g.: 2023, 2024, 2025",
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
            " [3] By Year & Month "
        } else {
            " [3] Year & Month "
        });

    let card3_inner = card3_block.inner(card_chunks[2]);
    f.render_widget(card3_block, card_chunks[2]);

    let card3_badge = if is_months_selected {
        Span::styled(
            " [ ★ ACTIVE ] ",
            Style::default()
                .bg(Theme::SUCCESS)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " [ Key 3 ] ",
            Style::default().fg(Theme::TEXT_MUTED),
        )
    };

    let card3_lines = vec![
        Line::from(vec![
            Span::styled("◷ Full Hierarchy", if is_months_selected {
                Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_MAIN)
            }),
            Span::raw("  "),
            card3_badge,
        ]),
        Line::from(Span::styled(
            "Structure: [Mailbox] / <Year> / <Month>",
            if is_months_selected {
                Style::default().fg(Theme::TEXT_MAIN)
            } else {
                Style::default().fg(Theme::TEXT_MUTED)
            },
        )),
        Line::from(Span::styled(
            "Maximum chronological organization",
            Style::default().fg(Theme::TEXT_MUTED),
        )),
        Line::from(Span::styled(
            "✓ E.g.: 2024 / 01-January, 2024 / 02-February",
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
    let has_date_filter = !state.routing_all_years || !state.routing_all_months;
    let filter_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if has_date_filter {
            Style::default().fg(Theme::WARNING)
        } else {
            Style::default().fg(Theme::ACCENT_SECONDARY)
        })
        .title(" Date Filter / Time Scope (Optional) ");

    let filter_inner = filter_block.inner(chunks[1]);
    f.render_widget(filter_block, chunks[1]);

    let filter_spans = if has_date_filter {
        let years_desc = state.format_years_filter_display();
        let months_desc = state.format_months_filter_display();
        let filter_desc = format!("{} • {}", years_desc, months_desc);

        vec![
            Span::styled("  ※ Active Filter: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("Only emails from: {} ", filter_desc),
                Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" • ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[A]", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" Years", Style::default().fg(Theme::TEXT_MAIN)),
            Span::styled("  •  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[M]", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" Months", Style::default().fg(Theme::TEXT_MAIN)),
            Span::styled("  •  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[R]", Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD)),
            Span::styled(" Clear Filters (Full History)", Style::default().fg(Theme::TEXT_MUTED)),
        ]
    } else {
        vec![
            Span::styled("  ✓ Scope: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                "Full History (No date filter — All emails will be imported) ",
                Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" • Press ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[A]", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" to filter Years  •  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[M]", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" to filter Months (H1 / H2 / Multi-select)", Style::default().fg(Theme::TEXT_MUTED)),
        ]
    };

    f.render_widget(Paragraph::new(Line::from(filter_spans)), filter_inner);

    // =========================================================================
    // 3. VISTA PREVIA DEL ENRUTAMIENTO MAPI EN VIVO
    // =========================================================================
    let selected_mboxes = state.selected_mailboxes();
    let mbox_names = if selected_mboxes.is_empty() {
        "Default mailbox".to_string()
    } else if selected_mboxes.len() == 1 {
        selected_mboxes[0].display_name.clone()
    } else {
        selected_mboxes.iter().map(|m| m.display_name.as_str()).collect::<Vec<_>>().join(" | ")
    };

    let preview_title = format!(" Live MAPI Routing Preview into: [{}] ", mbox_names);
    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(preview_title);

    let preview_inner = preview_block.inner(chunks[2]);
    f.render_widget(preview_block, chunks[2]);

    let preview_lines = match state.routing_granularity {
        RoutingGranularity::Mirror => {
            let filter_hint = if has_date_filter {
                format!(
                    "ℹ Active filter: Only emails from [{}] and [{}] will be copied to their native folders.",
                    state.format_years_filter_display(),
                    state.format_months_filter_display()
                )
            } else {
                "ℹ Full history: all emails transferred directly to matching folders without filter.".to_string()
            };

            vec![
                Line::from(Span::styled(
                    "Preserve Original Structure: PST folders are transferred directly to matching folders in Outlook:",
                    Style::default().fg(Theme::TEXT_MUTED),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  ◈ ", Style::default().fg(Theme::BRAND_PRIMARY)),
                    Span::styled(format!("[{}]", mbox_names), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled("      ├── ✉ Inbox /   ", Style::default().fg(Theme::ACCENT_PRIMARY)),
                    Span::styled("──▶  Direct mapping to mailbox (no date folders)", Style::default().fg(Theme::TEXT_MUTED)),
                ]),
                Line::from(vec![
                    Span::styled("      ├── ✉ Sent Items /   ", Style::default().fg(Theme::ACCENT_PRIMARY)),
                    Span::styled("──▶  Direct mapping to mailbox (no date folders)", Style::default().fg(Theme::TEXT_MUTED)),
                ]),
                Line::from(vec![
                    Span::styled("      └── ▸ PST Folders /     ", Style::default().fg(Theme::ACCENT_PRIMARY)),
                    Span::styled("──▶  Preserves exact folder name and content", Style::default().fg(Theme::TEXT_MUTED)),
                ]),
                Line::from(""),
                Line::from(Span::styled(filter_hint, Style::default().fg(Theme::SUCCESS))),
            ]
        }
        RoutingGranularity::Years => {
            let years_to_show: Vec<u32> = if !state.routing_all_years && !state.selected_years.is_empty() {
                state.selected_years.iter().copied().collect()
            } else {
                state.available_years_from_psts()
            };

            let mut lines = vec![
                Line::from(Span::styled(
                    if state.routing_all_years {
                        "Automatic organization of all years into target mailbox:".to_string()
                    } else {
                        format!("Exclusive organization for selected years [{}]:", state.format_years_filter_display())
                    },
                    Style::default().fg(Theme::TEXT_MUTED),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  ◈ ", Style::default().fg(Theme::BRAND_PRIMARY)),
                    Span::styled(format!("[{}]", mbox_names), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                ]),
            ];

            let count = years_to_show.len();
            for (i, y) in years_to_show.iter().take(4).enumerate() {
                let is_last = (i == count - 1) || (i == 3 && count <= 4);
                let branch = if is_last && count <= 4 { "      └── ◷ " } else { "      ├── ◷ " };
                lines.push(Line::from(vec![
                    Span::styled(format!("{}{:<5} /              ──▶  ", branch, y), Style::default().fg(Theme::ACCENT_PRIMARY)),
                    Span::styled(format!("[{}] / {}", mbox_names, y), Style::default().fg(Theme::TEXT_MAIN)),
                ]));
            }
            if count > 4 {
                lines.push(Line::from(vec![
                    Span::styled("      └── ◷ ... (more years)     ──▶  ", Style::default().fg(Theme::ACCENT_PRIMARY)),
                    Span::styled(format!("[{}] / <Year>", mbox_names), Style::default().fg(Theme::TEXT_MUTED)),
                ]));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                if !state.routing_all_months {
                    format!("✓ Active month filter: {}", state.format_months_filter_display())
                } else {
                    "✓ Each email will be organized into the corresponding annual folder by received date.".to_string()
                },
                Style::default().fg(Theme::SUCCESS),
            )));
            lines
        }
        RoutingGranularity::YearsAndMonths => {
            let years_to_show: Vec<u32> = if !state.routing_all_years && !state.selected_years.is_empty() {
                state.selected_years.iter().copied().collect()
            } else {
                state.available_years_from_psts()
            };
            let months_to_show: Vec<u32> = if !state.routing_all_months && !state.selected_months.is_empty() {
                state.selected_months.iter().copied().collect()
            } else {
                state.available_months_from_psts()
            };

            let mut lines = vec![
                Line::from(Span::styled(
                    format!(
                        "2-level hierarchical structure (Year / Month) for [{}]:",
                        state.format_years_filter_display()
                    ),
                    Style::default().fg(Theme::TEXT_MUTED),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  ◈ ", Style::default().fg(Theme::BRAND_PRIMARY)),
                    Span::styled(format!("[{}]", mbox_names), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                ]),
            ];

            let sample_year = years_to_show.first().copied().unwrap_or(2024);
            lines.push(Line::from(vec![
                Span::styled(format!("      └── ◷ {} /", sample_year), Style::default().fg(Theme::ACCENT_PRIMARY)),
            ]));

            let m_count = months_to_show.len();
            for (i, &m) in months_to_show.iter().take(3).enumerate() {
                let is_last = (i == m_count - 1) || (i == 2 && m_count <= 3);
                let branch = if is_last && m_count <= 3 { "          └── ▸ " } else { "          ├── ▸ " };
                let m_name = MONTH_NAMES.get((m as usize).saturating_sub(1)).unwrap_or(&"Month");
                lines.push(Line::from(vec![
                    Span::styled(format!("{}{:<14} / ──▶  ", branch, m_name), Style::default().fg(Theme::ACCENT_SECONDARY)),
                    Span::styled(format!("[{}] / {} / {}", mbox_names, sample_year, m_name), Style::default().fg(Theme::TEXT_MAIN)),
                ]));
            }
            if m_count > 3 {
                lines.push(Line::from(vec![
                    Span::styled("          └── ▸ ... (more months)  ──▶  ", Style::default().fg(Theme::ACCENT_SECONDARY)),
                    Span::styled(format!("[{}] / {} / <Month>", mbox_names, sample_year), Style::default().fg(Theme::TEXT_MUTED)),
                ]));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("✓ Active filter: {} | {}", state.format_years_filter_display(), state.format_months_filter_display()),
                Style::default().fg(Theme::SUCCESS),
            )));
            lines
        }
    };
    f.render_widget(Paragraph::new(preview_lines), preview_inner);

    // =========================================================================
    // 4. BARRA DE ATAJOS Y NAVEGACIÓN
    // =========================================================================
    let mut help_spans = vec![
        Span::styled("[1/2/3] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Choose Routing   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[←/→] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Toggle   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[A] ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
        Span::styled("Years   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[M] ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
        Span::styled("Months   ", Style::default().fg(Theme::TEXT_MUTED)),
    ];

    if has_date_filter {
        help_spans.push(Span::styled("[R] ", Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD)));
        help_spans.push(Span::styled("Reset   ", Style::default().fg(Theme::TEXT_MUTED)));
    }

    help_spans.extend(vec![
        Span::styled("[Enter] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        Span::styled("Continue to Deduplication   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[Esc] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Back", Style::default().fg(Theme::TEXT_MUTED)),
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
    let modal_width = 82.min(area.width.saturating_sub(4));
    let modal_height = 20.min(area.height.saturating_sub(2));

    let x = area.x + (area.width.saturating_sub(modal_width)) / 2;
    let y = area.y + (area.height.saturating_sub(modal_height)) / 2;
    let modal_rect = Rect::new(x, y, modal_width, modal_height);

    f.render_widget(Clear, modal_rect);

    let (title, subtitle, hints) = match state.active_routing_modal {
        RoutingModal::Criterion => (
            "--- Routing Criterion ---".to_string(),
            "Choose how you want to structure folders in the destination mailbox:".to_string(),
            "1/2/3 or ↑/↓ choose | Enter confirm | Esc back".to_string(),
        ),
        RoutingModal::YearScope => (
            "--- Date Filter: Year Selection ---".to_string(),
            "Select one or more years to process (Dynamically detected from PST file):".to_string(),
            "↑/↓ Navigate | Space Toggle | T All years | Enter Confirm | Esc Back".to_string(),
        ),
        RoutingModal::SpecificYear => (
            "--- Date Filter: Specific Year ---".to_string(),
            "Enter the year to process (example: 2024):".to_string(),
            "0-9 type | Backspace delete | Enter confirm | Esc back".to_string(),
        ),
        RoutingModal::MonthScope => {
            let sub = if state.routing_all_years {
                "Select months for all history (Quick presets or individual selection):".to_string()
            } else {
                format!("Select months for years [{}] (Presets or individual selection):", state.format_years_filter_display())
            };
            (
                "--- Date Filter: Month Selection ---".to_string(),
                sub,
                "1 1st Half (Jan-Jun) | 2 2nd Half (Jul-Dec) | T All | Space Toggle | Enter Confirm | Esc Back".to_string(),
            )
        }
        RoutingModal::SpecificMonth => {
            let sub = if state.routing_all_years {
                "Select month to process:".to_string()
            } else {
                format!("Select month for [{}]:", state.format_years_filter_display())
            };
            (
                "--- Date Filter: Specific Month ---".to_string(),
                sub,
                "←/→ change month | Enter confirm | Esc back".to_string(),
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
            Constraint::Min(8),    // Opciones interactivas con lista dinámica
            Constraint::Length(1), // Atajos
        ])
        .split(inner);

    let header_p = Paragraph::new(vec![
        Line::from(Span::styled(subtitle, Style::default().fg(Theme::TEXT_MUTED))),
        Line::from(Span::styled(hints, Style::default().fg(Theme::ACCENT_PRIMARY))),
    ]);
    f.render_widget(header_p, chunks[0]);

    match state.active_routing_modal {
        RoutingModal::Criterion => {
            let opt_mirror = "  1. Preserve Original Structure (Native PST - No time grouping)";
            let opt_years = "  2. Group by Year (1 level: [Mailbox] / <Year>)";
            let opt_months = "  3. Group by Year & Month (Hierarchical: [Mailbox] / <Year> / <Month>)";

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
            let available_years = state.available_years_from_psts();
            let mut lines = Vec::new();

            // Opción 0: Todos los años
            let is_all_sel = state.routing_all_years;
            let prefix_all = if is_all_sel { "[✓]" } else { "[ ]" };
            let is_cursor_0 = state.routing_modal_year_cursor == 0;
            let label_all = format!(" {} All years (Full PST history)", prefix_all);
            lines.push(if is_cursor_0 {
                Line::from(vec![
                    Span::styled(" ▶ ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                    Span::styled(label_all, Style::default().bg(Theme::BRAND_PRIMARY).fg(Color::Black).add_modifier(Modifier::BOLD)),
                ])
            } else {
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled(label_all, if is_all_sel { Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
                ])
            });
            lines.push(Line::from(""));

            // Opciones 1..=N: Años específicos dinámicos
            for (idx, &y) in available_years.iter().enumerate() {
                let item_idx = idx + 1;
                let is_cursor = state.routing_modal_year_cursor == item_idx;
                let is_checked = !state.routing_all_years && state.selected_years.contains(&y);
                let prefix = if is_checked { "[✓]" } else { "[ ]" };

                let mut total_items_year = 0usize;
                for detail in state.pst_details_cache.values() {
                    if let Some(&cnt) = detail.counts_by_year.get(&y.to_string()) {
                        total_items_year += cnt;
                    }
                }
                let count_str = if total_items_year > 0 {
                    format!(" (~{} emails)", total_items_year)
                } else {
                    String::new()
                };

                let item_text = format!(" {} Year {}{}", prefix, y, count_str);
                lines.push(if is_cursor {
                    Line::from(vec![
                        Span::styled(" ▶ ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                        Span::styled(item_text, Style::default().bg(Theme::BRAND_PRIMARY).fg(Color::Black).add_modifier(Modifier::BOLD)),
                    ])
                } else {
                    Line::from(vec![
                        Span::raw("   "),
                        Span::styled(item_text, if is_checked { Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MAIN) }),
                    ])
                });
            }

            let opts_p = Paragraph::new(lines);
            f.render_widget(opts_p, chunks[2]);
        }
        RoutingModal::SpecificYear => {
            let input_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
                .title(" Year to process (4 digits) ");

            let year_text = format!("  {} █", state.routing_input_year);
            let input_p = Paragraph::new(Line::from(Span::styled(
                year_text,
                Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD),
            )))
            .block(input_block);
            f.render_widget(input_p, chunks[2]);
        }
        RoutingModal::MonthScope => {
            let available_months = state.available_months_from_psts();
            let mut lines = Vec::new();

            // Fila 0: Todos los meses
            let is_all_sel = state.routing_all_months;
            let is_cursor_0 = state.routing_modal_month_cursor == 0;
            let prefix_all = if is_all_sel { "[✓]" } else { "[ ]" };
            let label_all = format!(" {} [T] All months (Full history)", prefix_all);
            lines.push(if is_cursor_0 {
                Line::from(vec![
                    Span::styled(" ▶ ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                    Span::styled(label_all, Style::default().bg(Theme::BRAND_PRIMARY).fg(Color::Black).add_modifier(Modifier::BOLD)),
                ])
            } else {
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled(label_all, if is_all_sel { Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
                ])
            });

            // Fila 1: Preset Primera Mitad (1..=6)
            let is_h1 = state.is_first_half_selected();
            let is_cursor_1 = state.routing_modal_month_cursor == 1;
            let prefix_h1 = if is_h1 { "[✓]" } else { "[ ]" };
            let label_h1 = format!(" {} [1] First half of year (01 - January to 06 - June / H1)", prefix_h1);
            lines.push(if is_cursor_1 {
                Line::from(vec![
                    Span::styled(" ▶ ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                    Span::styled(label_h1, Style::default().bg(Theme::BRAND_PRIMARY).fg(Color::Black).add_modifier(Modifier::BOLD)),
                ])
            } else {
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled(label_h1, if is_h1 { Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MAIN) }),
                ])
            });

            // Fila 2: Preset Segunda Mitad (7..=12)
            let is_h2 = state.is_second_half_selected();
            let is_cursor_2 = state.routing_modal_month_cursor == 2;
            let prefix_h2 = if is_h2 { "[✓]" } else { "[ ]" };
            let label_h2 = format!(" {} [2] Second half of year (07 - July to 12 - December / H2)", prefix_h2);
            lines.push(if is_cursor_2 {
                Line::from(vec![
                    Span::styled(" ▶ ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                    Span::styled(label_h2, Style::default().bg(Theme::BRAND_PRIMARY).fg(Color::Black).add_modifier(Modifier::BOLD)),
                ])
            } else {
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled(label_h2, if is_h2 { Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MAIN) }),
                ])
            });

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("  ── Available months in PST (Custom multi-selection) ──", Style::default().fg(Theme::TEXT_MUTED))));

            // Opciones 3..=N: Meses dinámicos
            for (idx, &m) in available_months.iter().enumerate() {
                let item_idx = idx + 3;
                let is_cursor = state.routing_modal_month_cursor == item_idx;
                let is_checked = !state.routing_all_months && state.selected_months.contains(&m);
                let prefix = if is_checked { "[✓]" } else { "[ ]" };
                let m_name = MONTH_NAMES.get((m as usize).saturating_sub(1)).unwrap_or(&"Month");

                let item_text = format!(" {} {}", prefix, m_name);
                lines.push(if is_cursor {
                    Line::from(vec![
                        Span::styled(" ▶ ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                        Span::styled(item_text, Style::default().bg(Theme::BRAND_PRIMARY).fg(Color::Black).add_modifier(Modifier::BOLD)),
                    ])
                } else {
                    Line::from(vec![
                        Span::raw("   "),
                        Span::styled(item_text, if is_checked { Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MAIN) }),
                    ])
                });
            }

            let opts_p = Paragraph::new(lines);
            f.render_widget(opts_p, chunks[2]);
        }
        RoutingModal::SpecificMonth => {
            let month_name = MONTH_NAMES.get((state.routing_input_month as usize).saturating_sub(1)).unwrap_or(&"01 - January");
            let year_label = state.format_years_filter_display();
            let sel_text = format!("  Years: {}  |  Selected month: [◀ {} ▶]", year_label, month_name);
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
