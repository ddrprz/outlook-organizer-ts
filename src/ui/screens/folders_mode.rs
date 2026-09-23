use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{AppState, CheckboxState, TransferMode},
    ui::theme::Theme,
};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(58), // Panel Izquierdo: Explorador de Carpetas
            Constraint::Percentage(42), // Panel Derecho: Modo de Transferencia
        ])
        .split(area);

    // 1. Panel Explorador de Carpetas
    let folders_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" ◈ Carpetas del PST a Importar ");

    let folders_inner = folders_block.inner(chunks[0]);
    f.render_widget(folders_block, chunks[0]);

    if state.is_inspecting_selected_psts() {
        let loading_lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  ⏳ Obteniendo carpetas internas del archivo PST...", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Analizando jerarquía de carpetas, conteos y tamaños en disco.", Style::default().fg(Theme::TEXT_MUTED)),
            ]),
            Line::from(vec![
                Span::styled("  Por favor espere un momento...", Style::default().fg(Theme::TEXT_MUTED)),
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
            folder_lines.push(Line::from(Span::styled("  No hay carpetas disponibles.", Style::default().fg(Theme::TEXT_MUTED))));
        } else {
            for (v_idx, &node_idx) in visible_indices.iter().enumerate() {
                if let Some(node) = state.folder_tree.nodes.get(node_idx) {
                    let is_focused = v_idx == state.folder_tree.selected_idx;
                    let cb_state = state.folder_tree.checkbox_state(node_idx);

                    // Formato de checkbox de ancho uniforme (6 caracteres)
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
                    let size_str = if node.size_mb <= 0.0 {
                        "0 MB".to_string()
                    } else if node.size_mb >= 1024.0 {
                        format!("{:.1} GB", node.size_mb / 1024.0)
                    } else {
                        format!("{:.1} MB", node.size_mb)
                    };

                    // Anchos de columnas derechas
                    let right_count = format!("{:>7}", count_str);
                    let right_size = format!("{:>10}", size_str);
                    let right_cols_len = 7 + 3 + 10; // 20 caracteres

                    // Estimación de ancho visual izquierdo:
                    // indent (level * 3) + cb (6) + icon 📁 (2 visible cols + 1 space = 3) + name_with_indicator
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
                        Span::raw("📁 "),
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
            Span::styled("[E] ", Style::default().fg(Theme::ACCENT_PRIMARY)),
            Span::styled("Desplegar  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[Espacio] ", Style::default().fg(Theme::ACCENT_PRIMARY)),
            Span::styled("Seleccionar  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[A] ", Style::default().fg(Theme::ACCENT_PRIMARY)),
            Span::styled("Todas  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[N] ", Style::default().fg(Theme::ACCENT_PRIMARY)),
            Span::styled("Ninguna", Style::default().fg(Theme::TEXT_MUTED)),
        ]));

        f.render_widget(Paragraph::new(folder_lines), folders_inner);
    }

    // 2. Panel Modo de Transferencia
    let mode_border_color = match state.transfer_mode {
        TransferMode::Copy => Theme::SUCCESS,
        TransferMode::Move => Theme::ACCENT_PRIMARY,
    };

    let mode_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(mode_border_color))
        .title(" Acción sobre los Correos ");

    let mode_inner = mode_block.inner(chunks[1]);
    f.render_widget(mode_block, chunks[1]);

    let radio_copy = if state.transfer_mode == TransferMode::Copy {
        Line::from(vec![
            Span::styled("[●] MODO COPIAR ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
            Span::styled("(Recomendado)", Style::default().fg(Theme::TEXT_MAIN)),
        ])
    } else {
        Line::from(Span::styled("[○] MODO COPIAR (PST intacto)", Style::default().fg(Theme::TEXT_MUTED)))
    };

    let radio_move = if state.transfer_mode == TransferMode::Move {
        Line::from(vec![
            Span::styled("[●] MODO MOVER ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("(Transferir y liberar espacio)", Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
        ])
    } else {
        Line::from(Span::styled("[○] MODO MOVER (Transferir correos)", Style::default().fg(Theme::TEXT_MUTED)))
    };

    let warning_note = if state.transfer_mode == TransferMode::Move {
        vec![
            Line::from(""),
            Line::from(Span::styled("ℹ️ ADVERTENCIA:", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("Esta opción transferirá los correos al buzón destino", Style::default().fg(Theme::TEXT_MAIN))),
            Line::from(Span::styled("y los eliminará del archivo PST tras confirmar su guardado seguro.", Style::default().fg(Theme::TEXT_MAIN))),
            Line::from(""),
            Line::from(Span::styled("✓ Proceso seguro: parada transaccional ante cualquier interrupción.", Style::default().fg(Theme::TEXT_MUTED))),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled("✓ Seguro y no destructivo:", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("Los correos se importan al buzón y el archivo PST", Style::default().fg(Theme::TEXT_MAIN))),
            Line::from(Span::styled("permanece exactamente intacto en modo solo lectura.", Style::default().fg(Theme::TEXT_MAIN))),
        ]
    };

    let mut mode_lines = vec![
        Line::from("Elija cómo interactuar con los elementos del archivo:"),
        Line::from(""),
        radio_copy,
        Line::from(""),
        radio_move,
    ];
    mode_lines.extend(warning_note);
    mode_lines.push(Line::from(""));
    mode_lines.push(Line::from(vec![
        Span::styled("[M] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Alternar Copiar / Mover", Style::default().fg(Theme::TEXT_MAIN)),
    ]));

    f.render_widget(Paragraph::new(mode_lines), mode_inner);
}
