use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{AppState, TransferMode},
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

    if visible_indices.is_empty() {
        folder_lines.push(Line::from(Span::styled("  No hay carpetas disponibles.", Style::default().fg(Theme::TEXT_MUTED))));
    } else {
        for (v_idx, &node_idx) in visible_indices.iter().enumerate() {
            if let Some(node) = state.folder_tree.nodes.get(node_idx) {
                let is_focused = v_idx == state.folder_tree.selected_idx;
                let indent = "   ".repeat(node.level);

                let expand_icon = if node.has_children {
                    if node.expanded { "▼ " } else { "▶ " }
                } else {
                    "• "
                };

                let checkbox = if node.selected { "[x] " } else { "[ ] " };
                let prefix = if is_focused { "▶ " } else { "  " };

                let mut spans = Vec::new();
                spans.push(Span::styled(
                    prefix,
                    Style::default().fg(if is_focused { Theme::ACCENT_PRIMARY } else { Theme::TEXT_MUTED }),
                ));
                spans.push(Span::raw(indent));
                spans.push(Span::styled(
                    expand_icon,
                    Style::default().fg(if node.has_children { Theme::ACCENT_PRIMARY } else { Theme::TEXT_MUTED }),
                ));
                spans.push(Span::styled(
                    checkbox,
                    Style::default().fg(if node.selected { Theme::SUCCESS } else { Theme::TEXT_MUTED })
                        .add_modifier(if node.selected { Modifier::BOLD } else { Modifier::empty() }),
                ));

                let name_style = if is_focused {
                    Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)
                } else if node.selected {
                    Style::default().fg(Theme::TEXT_MAIN)
                } else {
                    Style::default().fg(Theme::TEXT_MUTED)
                };
                spans.push(Span::styled(&node.name, name_style));

                if node.count > 0 {
                    spans.push(Span::styled(
                        format!(" ({} correos)", node.count),
                        Style::default().fg(if is_focused { Theme::TEXT_MAIN } else { Theme::TEXT_MUTED }),
                    ));
                }

                folder_lines.push(Line::from(spans));
            }
        }
    }

    folder_lines.push(Line::from(""));
    folder_lines.push(Line::from(vec![
        Span::styled("[↑/↓] ", Style::default().fg(Theme::ACCENT_PRIMARY)),
        Span::styled("Navegar  ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[E/Enter] ", Style::default().fg(Theme::ACCENT_PRIMARY)),
        Span::styled("Desplegar  ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[Espacio] ", Style::default().fg(Theme::ACCENT_PRIMARY)),
        Span::styled("Seleccionar  ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[A] ", Style::default().fg(Theme::ACCENT_PRIMARY)),
        Span::styled("Todas  ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[N] ", Style::default().fg(Theme::ACCENT_PRIMARY)),
        Span::styled("Ninguna", Style::default().fg(Theme::TEXT_MUTED)),
    ]));

    f.render_widget(Paragraph::new(folder_lines), folders_inner);

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
