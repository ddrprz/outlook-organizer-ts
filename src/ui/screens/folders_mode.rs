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
            Constraint::Percentage(50), // Panel Izquierdo: Carpetas a Importar
            Constraint::Percentage(50), // Panel Derecho: Modo de Transferencia
        ])
        .split(area);

    // 1. Panel Carpetas
    let folders_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Carpetas de Origen ");

    let folders_inner = folders_block.inner(chunks[0]);
    f.render_widget(folders_block, chunks[0]);

    let f_inbox = if state.include_inbox { "[x] Bandeja de entrada (Inbox)" } else { "[ ] Bandeja de entrada" };
    let f_sent = if state.include_sent { "[x] Elementos enviados (Sent Items)" } else { "[ ] Elementos enviados" };
    let f_deleted = if state.include_deleted { "[x] Elementos eliminados (Deleted)" } else { "[ ] Elementos eliminados" };
    let f_custom = if state.include_custom_folders { "[x] Carpetas personalizadas / subcarpetas" } else { "[ ] Carpetas personalizadas" };

    let folders_lines = vec![
        Line::from("Seleccione las carpetas del PST a procesar:"),
        Line::from(""),
        Line::from(Span::styled(f_inbox, Style::default().fg(if state.include_inbox { Theme::SUCCESS } else { Theme::TEXT_MUTED }))),
        Line::from(Span::styled(f_sent, Style::default().fg(if state.include_sent { Theme::SUCCESS } else { Theme::TEXT_MUTED }))),
        Line::from(Span::styled(f_deleted, Style::default().fg(if state.include_deleted { Theme::SUCCESS } else { Theme::TEXT_MUTED }))),
        Line::from(Span::styled(f_custom, Style::default().fg(if state.include_custom_folders { Theme::SUCCESS } else { Theme::TEXT_MUTED }))),
        Line::from(""),
        Line::from(Span::styled("[1-4] Alternar carpeta seleccionada", Style::default().fg(Theme::TEXT_MUTED))),
    ];
    f.render_widget(Paragraph::new(folders_lines), folders_inner);

    // 2. Panel Modo de Transferencia
    let mode_border_color = match state.transfer_mode {
        TransferMode::Copy => Theme::SUCCESS,
        TransferMode::Move => Theme::WARNING,
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
            Span::styled("(Recomendado - PST permanece intacto)", Style::default().fg(Theme::TEXT_MAIN)),
        ])
    } else {
        Line::from(Span::styled("[○] MODO COPIAR (PST intacto)", Style::default().fg(Theme::TEXT_MUTED)))
    };

    let radio_move = if state.transfer_mode == TransferMode::Move {
        Line::from(vec![
            Span::styled("[●] MODO MOVER ", Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD)),
            Span::styled("(Destructivo - Borra del PST tras éxito)", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
        ])
    } else {
        Line::from(Span::styled("[○] MODO MOVER (Eliminar correos del PST)", Style::default().fg(Theme::TEXT_MUTED)))
    };

    let warning_note = if state.transfer_mode == TransferMode::Move {
        vec![
            Line::from(""),
            Line::from(Span::styled("⚠️ ATENCIÓN: El modo mover eliminará correos del archivo PST.", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("La eliminación solo ocurre tras confirmación de guardado seguro.", Style::default().fg(Theme::TEXT_MUTED))),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled("✓ Seguro: El PST se mantendrá en modo solo lectura.", Style::default().fg(Theme::SUCCESS))),
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
    mode_lines.push(Line::from(Span::styled("[M] Alternar Copiar / Mover", Style::default().fg(Theme::ACCENT_PRIMARY))));

    f.render_widget(Paragraph::new(mode_lines), mode_inner);
}
