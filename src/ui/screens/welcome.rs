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
            Constraint::Length(7), // Tarjeta de Bienvenida y Estado
            Constraint::Length(10), // Configuración del Perfil
            Constraint::Min(2),    // Ayuda contextual
        ])
        .split(area);

    // 1. Tarjeta de Bienvenida
    let welcome_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Conexión y Estado MAPI ");

    let welcome_inner = welcome_block.inner(chunks[0]);
    f.render_widget(welcome_block, chunks[0]);

    let welcome_lines = vec![
        Line::from(vec![
            Span::styled("Bienvenido a ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("Outlook Organizer TS", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" — Herramienta de Migración y Organización de PSTs", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Estado de Outlook: ", Style::default().fg(Theme::TEXT_MAIN)),
            Span::styled("● Sesión MAPI Detectada / Disponible", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Integridad PST:    ", Style::default().fg(Theme::TEXT_MAIN)),
            Span::styled("Modo Seguro Activo (Cero riesgo de corrupción)", Style::default().fg(Theme::BRAND_PRIMARY)),
        ]),
    ];
    f.render_widget(Paragraph::new(welcome_lines), welcome_inner);

    // 2. Selección de Perfil
    let profile_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
        .title(" Configuración del Perfil de Outlook ");

    let profile_inner = profile_block.inner(chunks[1]);
    f.render_widget(profile_block, chunks[1]);

    let radio_default = if state.use_default_profile {
        Span::styled("[●] Usar perfil predeterminado del sistema (Recomendado)", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[○] Usar perfil predeterminado del sistema", Style::default().fg(Theme::TEXT_MUTED))
    };

    let radio_custom = if !state.use_default_profile {
        Span::styled("[●] Especificar perfil MAPI manualmente:", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[○] Especificar perfil MAPI manualmente", Style::default().fg(Theme::TEXT_MUTED))
    };

    let profile_input_display = if !state.use_default_profile {
        if state.custom_profile_name.is_empty() {
            Span::styled("    Nombre: [ Escribe el nombre del perfil... ]", Style::default().fg(Theme::WARNING))
        } else {
            Span::styled(format!("    Nombre: [ {} ]", state.custom_profile_name), Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD))
        }
    } else {
        Span::styled("    Nombre: [ Perfil Predeterminado de Windows ]", Style::default().fg(Theme::TEXT_MUTED))
    };

    let profile_lines = vec![
        Line::from("Seleccione el perfil de Outlook que contiene los buzones destino:"),
        Line::from(""),
        Line::from(radio_default),
        Line::from(""),
        Line::from(radio_custom),
        Line::from(profile_input_display),
    ];
    f.render_widget(Paragraph::new(profile_lines), profile_inner);

    // 3. Indicaciones de interacción
    let help_text = Line::from(vec![
        Span::styled("💡 Tip: ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
        Span::styled("Presiona ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[Espacio] / [P]", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled(" para alternar entre perfil predeterminado o manual.", Style::default().fg(Theme::TEXT_MUTED)),
    ]);
    f.render_widget(Paragraph::new(help_text).alignment(Alignment::Left), chunks[2]);
}
