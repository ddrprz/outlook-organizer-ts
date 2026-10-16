use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{AppState, CheckboxState, RoutingGranularity, RoutingModal, TransferMode},
    ui::{format::format_size_mb, screens::routing::render_routing_modal, theme::Theme},
};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(52), // Panel Izquierdo: Explorador de Carpetas
            Constraint::Percentage(48), // Panel Derecho: Reglas, Enrutamiento y Políticas
        ])
        .split(area);

    // =========================================================================
    // 1. PANEL IZQUIERDO: EXPLORADOR JERÁRQUICO DE CARPETAS DEL PST
    // =========================================================================
    let folders_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" ◈ Carpetas del PST a Importar ");

    let folders_inner = folders_block.inner(main_chunks[0]);
    f.render_widget(folders_block, main_chunks[0]);

    if state.is_inspecting_selected_psts() {
        let loading_lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  ⧗ Inspeccionando estructura interna del archivo PST...",
                    Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  Analizando jerarquía de carpetas, conteo de correos y tamaños en disco.",
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  Por favor espere un momento...",
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
            ]),
        ];
        f.render_widget(Paragraph::new(loading_lines), folders_inner);
    } else {
        let visible_indices = state.folder_tree.visible_indices();
        let selected_count = state.folder_tree.nodes.iter().filter(|n| n.selected).count();
        let total_nodes = state.folder_tree.nodes.len();

        let mut folder_lines = vec![
            Line::from(vec![
                Span::styled("Estructura de carpetas ", Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
                Span::styled(format!("({} de {} seleccionadas):", selected_count, total_nodes), Style::default().fg(Theme::TEXT_MUTED)),
            ]),
            Line::from(""),
        ];

        let content_width = folders_inner.width as usize;

        if visible_indices.is_empty() {
            folder_lines.push(Line::from(Span::styled("  No hay carpetas disponibles para mostrar.", Style::default().fg(Theme::TEXT_MUTED))));
        } else {
            for (v_idx, &node_idx) in visible_indices.iter().enumerate() {
                if let Some(node) = state.folder_tree.nodes.get(node_idx) {
                    let is_focused = v_idx == state.folder_tree.selected_idx;
                    let cb_state = state.folder_tree.checkbox_state(node_idx);

                    let cb_span = match cb_state {
                        CheckboxState::Unchecked => Span::styled("[ ]   ", Style::default().fg(Theme::TEXT_MUTED)),
                        CheckboxState::Checked => Span::styled("[x]   ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
                        CheckboxState::Indeterminate => Span::styled("[ - ] ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
                    };

                    let indent_str = "   ".repeat(node.level);

                    let name_with_indicator = if node.has_children {
                        if node.expanded {
                            format!("{} ▼", node.name)
                        } else {
                            format!("{} ▶", node.name)
                        }
                    } else {
                        node.name.clone()
                    };

                    let name_style = if is_focused {
                        Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)
                    } else if node.selected || cb_state != CheckboxState::Unchecked {
                        Style::default().fg(Theme::TEXT_MAIN)
                    } else {
                        Style::default().fg(Theme::TEXT_MUTED)
                    };

                    let count_str = format!("{}", node.count);
                    let size_str = format_size_mb(node.size_mb);

                    let right_count = format!("{:>7}", count_str);
                    let right_size = format!("{:>10}", size_str);
                    let right_cols_len = 7 + 3 + 10;

                    let left_vis_len = (node.level * 3) + 6 + 3 + name_with_indicator.chars().count();
                    let padding = content_width.saturating_sub(left_vis_len + right_cols_len);
                    let pad_str = " ".repeat(padding.max(2));

                    let row_bg = if is_focused {
                        Color::Rgb(38, 42, 52)
                    } else {
                        Color::Reset
                    };

                    let line = Line::from(vec![
                        Span::raw(indent_str),
                        cb_span,
                        Span::raw("▸ "),
                        Span::styled(name_with_indicator, name_style),
                        Span::raw(pad_str),
                        Span::styled(right_count, Style::default().fg(if is_focused { Theme::TEXT_MAIN } else { Theme::TEXT_MUTED })),
                        Span::raw("   "),
                        Span::styled(right_size, Style::default().fg(if is_focused { Theme::TEXT_MAIN } else { Theme::TEXT_MUTED })),
                    ]).style(Style::default().bg(row_bg));

                    folder_lines.push(line);
                }
            }
        }

        folder_lines.push(Line::from(""));
        folder_lines.push(Line::from(vec![
            Span::styled("[↑/↓] ", Style::default().fg(Theme::ACCENT_PRIMARY)),
            Span::styled("Navegar  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[Espacio] ", Style::default().fg(Theme::ACCENT_PRIMARY)),
            Span::styled("Marcar  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[E] ", Style::default().fg(Theme::ACCENT_PRIMARY)),
            Span::styled("Expandir  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[A/N] ", Style::default().fg(Theme::ACCENT_PRIMARY)),
            Span::styled("Todos/Ninguno", Style::default().fg(Theme::TEXT_MUTED)),
        ]));

        f.render_widget(Paragraph::new(folder_lines), folders_inner);
    }

    // =========================================================================
    // 2. PANEL DERECHO: REGLAS, ENRUTAMIENTO, DUPLICADOS Y RENDIMIENTO
    // =========================================================================
    let is_tall = main_chunks[1].height >= 26;
    let right_constraints = if is_tall {
        vec![
            Constraint::Length(7), // 1. Modo de Transferencia
            Constraint::Length(8), // 2. Enrutamiento Temporal
            Constraint::Length(7), // 3. Deduplicación y Throttling
            Constraint::Min(4),    // 4. Preset Rápido y Ayuda
        ]
    } else {
        vec![
            Constraint::Length(6), // 1. Modo de Transferencia
            Constraint::Length(7), // 2. Enrutamiento Temporal
            Constraint::Length(6), // 3. Deduplicación y Throttling
            Constraint::Min(3),    // 4. Preset Rápido y Ayuda
        ]
    };

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(right_constraints)
        .split(main_chunks[1]);

    // -------------------------------------------------------------------------
    // 2.1 MODO DE TRANSFERENCIA (Copiar vs Mover)
    // -------------------------------------------------------------------------
    let mode_border_color = match state.transfer_mode {
        TransferMode::Copy => Theme::SUCCESS,
        TransferMode::Move => Theme::WARNING,
    };

    let mode_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(mode_border_color))
        .title(" Acción sobre los Correos ");

    let mode_inner = mode_block.inner(right_chunks[0]);
    f.render_widget(mode_block, right_chunks[0]);

    let radio_copy = if state.transfer_mode == TransferMode::Copy {
        Line::from(vec![
            Span::styled(" [●] MODO COPIAR ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
            Span::styled("— Recomendado (PST original intacto en sólo lectura)", Style::default().fg(Theme::TEXT_MAIN)),
        ])
    } else {
        Line::from(Span::styled(" [○] MODO COPIAR (Sin alterar archivos originales)", Style::default().fg(Theme::TEXT_MUTED)))
    };

    let radio_move = if state.transfer_mode == TransferMode::Move {
        Line::from(vec![
            Span::styled(" [●] MODO MOVER ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
            Span::styled("— Transfiere y elimina del PST tras confirmación", Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
        ])
    } else {
        Line::from(Span::styled(" [○] MODO MOVER (Transfiere correos y libera espacio)", Style::default().fg(Theme::TEXT_MUTED)))
    };

    let mode_lines = vec![
        radio_copy,
        radio_move,
        Line::from(""),
        Line::from(vec![
            Span::styled(" Presione ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[M]", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" para alternar entre Copiar y Mover.", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];
    f.render_widget(Paragraph::new(mode_lines), mode_inner);

    // -------------------------------------------------------------------------
    // 2.2 ENRUTAMIENTO TEMPORAL POR CARPETAS
    // -------------------------------------------------------------------------
    let routing_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BRAND_PRIMARY))
        .title(" Organización Temporal (Enrutamiento MAPI) ");

    let routing_inner = routing_block.inner(right_chunks[1]);
    f.render_widget(routing_block, right_chunks[1]);

    let crit_1_style = if state.routing_granularity == RoutingGranularity::Mirror {
        Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Theme::TEXT_MUTED)
    };
    let crit_2_style = if state.routing_granularity == RoutingGranularity::Years {
        Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Theme::TEXT_MUTED)
    };
    let crit_3_style = if state.routing_granularity == RoutingGranularity::YearsAndMonths {
        Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Theme::TEXT_MUTED)
    };

    let date_range_desc = if !state.routing_all_years || !state.routing_all_months {
        let years_str = state.format_years_filter_display();
        let months_str = state.format_months_filter_display();
        format!("Filtro Activo: Años [{}] | Meses [{}]", years_str, months_str)
    } else {
        "Rango: Historial Completo (Todos los años y meses)".to_string()
    };

    let routing_lines = vec![
        Line::from(vec![
            Span::styled(if state.routing_granularity == RoutingGranularity::Mirror { " [●] " } else { " [○] " }, crit_1_style),
            Span::styled("[1] Estructura Original (1:1 nativa)  ", crit_1_style),
            Span::styled(if state.routing_granularity == RoutingGranularity::Years { " [●] " } else { " [○] " }, crit_2_style),
            Span::styled("[2] Por Años", crit_2_style),
        ]),
        Line::from(vec![
            Span::styled(if state.routing_granularity == RoutingGranularity::YearsAndMonths { " [●] " } else { " [○] " }, crit_3_style),
            Span::styled("[3] Por Año y Mes (ej: Bandeja / 2024 / 05-Mayo) ★ Recomendado", crit_3_style),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" • ", Style::default().fg(Theme::ACCENT_PRIMARY)),
            Span::styled(date_range_desc, Style::default().fg(Theme::TEXT_MAIN)),
        ]),
        Line::from(vec![
            Span::styled(" [1-3] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Elegir Criterio   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[F] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Filtrar Años/Meses", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];
    f.render_widget(Paragraph::new(routing_lines), routing_inner);

    // -------------------------------------------------------------------------
    // 2.3 POLÍTICAS DE DUPLICADOS Y RENDIMIENTO (THROTTLING)
    // -------------------------------------------------------------------------
    let is_online = state.is_target_online();

    let policies_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
        .title(" Políticas de Duplicados & Seguridad de Red ");

    let policies_inner = policies_block.inner(right_chunks[2]);
    f.render_widget(policies_block, right_chunks[2]);

    let dedup_badge = if state.deduplication_enabled {
        Span::styled("[x] Omitir Duplicados ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[ ] Omitir Duplicados ", Style::default().fg(Theme::WARNING))
    };

    let deep_badge = if is_online {
        Span::styled("[Modo Online M365: Revisión directa activa]", Style::default().fg(Theme::TEXT_MUTED))
    } else if state.deep_scan_enabled {
        Span::styled("[x] Revisión Profunda (Deep Scan)", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[ ] Revisión Profunda", Style::default().fg(Theme::TEXT_MUTED))
    };

    let thrott_badge = if state.adaptive_throttling_enabled {
        Span::styled("[x] Throttling Adaptativo Anti-429 (Activo)", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[ ] Throttling Desactivado (Riesgo de saturación M365)", Style::default().fg(Theme::WARNING))
    };

    let policies_lines = vec![
        Line::from(vec![
            Span::styled(" • Deduplicación: ", Style::default().fg(Theme::TEXT_MUTED)),
            dedup_badge,
            Span::styled(" | ", Style::default().fg(Theme::TEXT_MUTED)),
            deep_badge,
        ]),
        Line::from(vec![
            Span::styled(" • Rendimiento:   ", Style::default().fg(Theme::TEXT_MUTED)),
            thrott_badge,
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [D] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Duplicados On/Off  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[R] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Revisión Profunda  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[T] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Throttling On/Off", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];
    f.render_widget(Paragraph::new(policies_lines), policies_inner);

    // -------------------------------------------------------------------------
    // 2.4 PRESET RÁPIDO Y CONTINUAR
    // -------------------------------------------------------------------------
    let preset_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::SUCCESS))
        .title(" Resumen de Configuración ");

    let preset_inner = preset_block.inner(right_chunks[3]);
    f.render_widget(preset_block, right_chunks[3]);

    let preset_lines = vec![
        Line::from(vec![
            Span::styled(" [O] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
            Span::styled("Restaurar Configuración Óptima Recomendada ", Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
            Span::styled("(Copiar + Año/Mes + Duplicados + Throttling)", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" ➔ Presione ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[ENTER]", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
            Span::styled(" para avanzar al Paso 4 (Resumen y Confirmación).", Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
        ]),
    ];
    f.render_widget(Paragraph::new(preset_lines), preset_inner);

    // =========================================================================
    // 3. MODAL DE SELECCIÓN DE FECHAS (Años / Meses si está activo por la tecla F)
    // =========================================================================
    if state.active_routing_modal != RoutingModal::None {
        render_routing_modal(f, area, state);
    }
}
