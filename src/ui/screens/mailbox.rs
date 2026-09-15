use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{app::AppState, ui::theme::Theme};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Tipo de Buzón
            Constraint::Length(7), // Buzón Seleccionado
            Constraint::Min(2),    // Consejos
        ])
        .split(area);

    // 1. Selector tipo de buzón
    let type_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Tipo de Buzón Destino ");

    let type_inner = type_block.inner(chunks[0]);
    f.render_widget(type_block, chunks[0]);

    let radio_personal = if !state.is_shared_mailbox {
        Span::styled("[●] Buzón personal principal (Predeterminado de la cuenta)", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[○] Buzón personal principal", Style::default().fg(Theme::TEXT_MUTED))
    };

    let radio_shared = if state.is_shared_mailbox {
        Span::styled("[●] Buzón compartido (Shared Mailbox / Múltiples buzones)", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[○] Buzón compartido (Shared Mailbox / Múltiples buzones)", Style::default().fg(Theme::TEXT_MUTED))
    };

    let type_lines = vec![
        Line::from("Seleccione el destino hacia donde se transferirán los correos:"),
        Line::from(""),
        Line::from(radio_personal),
        Line::from(radio_shared),
    ];
    f.render_widget(Paragraph::new(type_lines), type_inner);

    // 2. Destino específico
    let target_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
        .title(" Buzón MAPI Activo ");

    let target_inner = target_block.inner(chunks[1]);
    f.render_widget(target_block, chunks[1]);

    let target_lines = vec![
        Line::from(vec![
            Span::styled("Dirección de destino: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(&state.target_mailbox, Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Permisos verificados: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("✓ Escritura y creación de subcarpetas MAPI concedidas", Style::default().fg(Theme::SUCCESS)),
        ]),
    ];
    f.render_widget(Paragraph::new(target_lines), target_inner);

    // 3. Ayuda
    let help_line = Line::from(vec![
        Span::styled("💡 [S] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Alternar entre Buzón Personal y Buzón Compartido", Style::default().fg(Theme::TEXT_MUTED)),
    ]);
    f.render_widget(Paragraph::new(help_line).alignment(Alignment::Center), chunks[2]);
}
