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
            Constraint::Length(6), // Criterio y Alcance configurado
            Constraint::Min(8),    // Mapeo hacia buzones y vista previa
            Constraint::Length(2), // Atajos
        ])
        .split(area);

    // 1. Criterio y Alcance Activo
    let summary_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Criterio de Enrutamiento Configurado ");

    let summary_inner = summary_block.inner(chunks[0]);
    f.render_widget(summary_block, chunks[0]);

    let gran_label = match state.routing_granularity {
        RoutingGranularity::Years => "Agrupado por Años (ej. 2023 -> Buzón A, 2024 -> Buzón B)",
        RoutingGranularity::YearsAndMonths => "Agrupado por Meses (ej. Enero 2024 -> Buzón A, Febrero 2024 -> Buzón B)",
    };

    let scope_label = if let Some(year) = state.specific_year {
        if let Some(m) = state.specific_month {
            let month_str = MONTH_NAMES.get((m as usize).saturating_sub(1)).unwrap_or(&"Mes desconocido");
            format!("Año {} / {}", year, month_str)
        } else {
            format!("Año específico: {}", year)
        }
    } else {
        match state.routing_granularity {
            RoutingGranularity::Years => "Todos los años (recomendado)".to_string(),
            RoutingGranularity::YearsAndMonths => "Todos los meses (recomendado)".to_string(),
        }
    };

    let summary_lines = vec![
        Line::from(vec![
            Span::styled("Estructura de agrupación: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(gran_label, Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Alcance temporal del PST:  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(scope_label, Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        ]),
    ];
    f.render_widget(Paragraph::new(summary_lines), summary_inner);

    // 2. Mapeo y Vista Previa
    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
        .title(" Vista Previa del Enrutamiento MAPI ");

    let preview_inner = preview_block.inner(chunks[1]);
    f.render_widget(preview_block, chunks[1]);

    let selected_mboxes = state.selected_mailboxes();
    let mbox_names = if selected_mboxes.is_empty() {
        "Buzón predeterminado".to_string()
    } else if selected_mboxes.len() == 1 {
        selected_mboxes[0].display_name.clone()
    } else {
        selected_mboxes.iter().map(|m| m.display_name.as_str()).collect::<Vec<_>>().join(" | ")
    };

    let preview_lines = match state.routing_granularity {
        RoutingGranularity::Years => vec![
            Line::from(Span::styled("Mapeo de correos por año hacia buzón(es) destino:", Style::default().fg(Theme::TEXT_MUTED))),
            Line::from(""),
            Line::from(vec![
                Span::styled("  📁 2023  ──▶  ", Style::default().fg(Theme::BRAND_PRIMARY)),
                Span::styled(format!("[{}] / 2023", mbox_names), Style::default().fg(Theme::TEXT_MAIN)),
            ]),
            Line::from(vec![
                Span::styled("  📁 2024  ──▶  ", Style::default().fg(Theme::BRAND_PRIMARY)),
                Span::styled(format!("[{}] / 2024", mbox_names), Style::default().fg(Theme::TEXT_MAIN)),
            ]),
            Line::from(vec![
                Span::styled("  📁 2025  ──▶  ", Style::default().fg(Theme::BRAND_PRIMARY)),
                Span::styled(format!("[{}] / 2025", mbox_names), Style::default().fg(Theme::TEXT_MAIN)),
            ]),
            Line::from(""),
            Line::from(Span::styled("✓ Los correos se clasificarán automáticamente según la fecha del mensaje.", Style::default().fg(Theme::SUCCESS))),
        ],
        RoutingGranularity::YearsAndMonths => vec![
            Line::from(Span::styled("Mapeo de correos por año y mes hacia buzón(es) destino:", Style::default().fg(Theme::TEXT_MUTED))),
            Line::from(""),
            Line::from(vec![
                Span::styled("  📁 2024 / 01-Enero   ──▶  ", Style::default().fg(Theme::BRAND_PRIMARY)),
                Span::styled(format!("[{}] / 2024 / 01-Enero", mbox_names), Style::default().fg(Theme::TEXT_MAIN)),
            ]),
            Line::from(vec![
                Span::styled("  📁 2024 / 02-Febrero ──▶  ", Style::default().fg(Theme::BRAND_PRIMARY)),
                Span::styled(format!("[{}] / 2024 / 02-Febrero", mbox_names), Style::default().fg(Theme::TEXT_MAIN)),
            ]),
            Line::from(vec![
                Span::styled("  📁 2024 / ...        ──▶  ", Style::default().fg(Theme::BRAND_PRIMARY)),
                Span::styled(format!("[{}] / 2024 / ...", mbox_names), Style::default().fg(Theme::TEXT_MAIN)),
            ]),
            Line::from(""),
            Line::from(Span::styled("✓ Máxima granularidad temporal para buzones compartidos o históricos.", Style::default().fg(Theme::SUCCESS))),
        ],
    };
    f.render_widget(Paragraph::new(preview_lines), preview_inner);

    // 3. Atajos
    let help_line = Line::from(vec![
        Span::styled("[C] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Cambiar Criterio / Reconfigurar   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[Enter] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        Span::styled("Continuar a Deduplicación   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[Esc] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Atrás", Style::default().fg(Theme::TEXT_MUTED)),
    ]);
    f.render_widget(Paragraph::new(help_line).alignment(Alignment::Center), chunks[2]);

    // 4. VENTANA MODAL (Si está activa)
    if state.active_routing_modal != RoutingModal::None {
        render_routing_modal(f, area, state);
    }
}

fn render_routing_modal(f: &mut Frame, area: Rect, state: &AppState) {
    let modal_width = 74.min(area.width.saturating_sub(4));
    let modal_height = 13.min(area.height.saturating_sub(2));

    let x = area.x + (area.width.saturating_sub(modal_width)) / 2;
    let y = area.y + (area.height.saturating_sub(modal_height)) / 2;
    let modal_rect = Rect::new(x, y, modal_width, modal_height);

    // Limpiar el fondo debajo del popup modal
    f.render_widget(Clear, modal_rect);

    let (title, subtitle, hints) = match state.active_routing_modal {
        RoutingModal::Criterion => (
            "--- Criterio de Enrutamiento ---",
            "Selecciona como deseas agrupar los correos para enviarlos a diferentes buzones.",
            "↑/↓ mover | Enter confirmar | Q cancelar",
        ),
        RoutingModal::Scope => {
            let sub = if state.routing_granularity == RoutingGranularity::Years {
                "Alcance de anios para el escaneo del PST."
            } else {
                "Alcance de meses para el escaneo del PST."
            };
            (
                "--- Escanear PST ---",
                sub,
                "↑/↓ mover | Enter confirmar | Q cancelar",
            )
        }
        RoutingModal::SpecificYear => (
            "--- Anio Especifico ---",
            "Ingrese el anio a procesar para el filtrado del PST:",
            "0-9 escribir | Backspace borrar | Enter confirmar | Esc volver",
        ),
        RoutingModal::SpecificMonth => (
            "--- Mes Especifico ---",
            "Seleccione el mes a procesar:",
            "←/→ cambiar mes | Enter confirmar | Esc volver",
        ),
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

    // 1. Subtítulo
    let header_p = Paragraph::new(vec![
        Line::from(Span::styled(subtitle, Style::default().fg(Theme::TEXT_MUTED))),
        Line::from(Span::styled(hints, Style::default().fg(Theme::ACCENT_PRIMARY))),
    ]);
    f.render_widget(header_p, chunks[0]);

    // 2. Opciones según el modal activo
    match state.active_routing_modal {
        RoutingModal::Criterion => {
            let opt_years = "  Agrupado por Anos  (Ej: 2023 -> Buzon A, 2024 -> Buzon B)";
            let opt_months = "  Agrupado por Meses (Ej: Enero 2024 -> Buzon A, Febrero 2024 -> Buzon B)";

            let line_years = if state.routing_modal_criterion_idx == 0 {
                Line::from(Span::styled(
                    opt_years,
                    Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(opt_years, Style::default().fg(Color::Gray)))
            };

            let line_months = if state.routing_modal_criterion_idx == 1 {
                Line::from(Span::styled(
                    opt_months,
                    Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(opt_months, Style::default().fg(Color::Gray)))
            };

            let opts_p = Paragraph::new(vec![
                line_years,
                Line::from(""),
                line_months,
            ]);
            f.render_widget(opts_p, chunks[2]);
        }
        RoutingModal::Scope => {
            let (opt_all, opt_spec) = if state.routing_granularity == RoutingGranularity::Years {
                ("  Todos los anios (recomendado)", "  Un anio especifico")
            } else {
                ("  Todos los meses (recomendado)", "  Un mes especifico")
            };

            let line_all = if state.routing_modal_scope_idx == 0 {
                Line::from(Span::styled(
                    opt_all,
                    Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(opt_all, Style::default().fg(Color::Gray)))
            };

            let line_spec = if state.routing_modal_scope_idx == 1 {
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
        RoutingModal::SpecificMonth => {
            let month_name = MONTH_NAMES.get((state.routing_input_month as usize).saturating_sub(1)).unwrap_or(&"01 - Enero");
            let sel_text = format!("  Año: {}  |  Mes seleccionado: [◀ {} ▶]", state.routing_input_year, month_name);
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
