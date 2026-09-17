use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::{app::AppState, ui::theme::Theme};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let has_warning = state.mailbox_warning_notice.is_some();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if has_warning {
            vec![
                Constraint::Length(5), // Resumen de perfil y buzones
                Constraint::Min(8),    // Tabla interactiva de buzones
                Constraint::Length(2), // Aviso de advertencia
                Constraint::Length(2), // Atajos
            ]
        } else {
            vec![
                Constraint::Length(5), // Resumen de perfil y buzones
                Constraint::Min(8),    // Tabla interactiva de buzones
                Constraint::Length(2), // Atajos
            ]
        })
        .split(area);

    // 1. Resumen de Perfil MAPI y Detección
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Detección de Buzones y Almacenes MAPI ");

    let header_inner = header_block.inner(chunks[0]);
    f.render_widget(header_block, chunks[0]);

    let profile_desc = if state.use_default_profile {
        "Perfil predeterminado del sistema".to_string()
    } else if state.custom_profile_name.trim().is_empty() {
        "Perfil manual (no especificado)".to_string()
    } else {
        format!("Perfil manual: {}", state.custom_profile_name)
    };

    let selected_count = state.selected_mailboxes().len();
    let total_count = state.discovered_mailboxes.len();

    let header_lines = vec![
        Line::from(vec![
            Span::styled("Perfil MAPI en uso: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(profile_desc, Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            if state.is_loading_mailboxes {
                Span::styled("   [⏳ Consultando Outlook...]", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD))
            } else {
                Span::styled("   [✓ Conexión establecida]", Style::default().fg(Theme::SUCCESS))
            },
        ]),
        Line::from(vec![
            Span::styled("Buzones encontrados: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!("{}", total_count), Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" | Marcados como destino: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", selected_count),
                Style::default().fg(if selected_count > 0 { Theme::SUCCESS } else { Theme::WARNING }).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                if selected_count > 1 { " (Modo multi-destino activo)" } else { " (Permite seleccionar uno o varios)" },
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(header_lines), header_inner);

    // 2. Tabla Interactiva de Buzones Destino
    if state.is_loading_mailboxes && state.discovered_mailboxes.is_empty() {
        let loading_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
            .title(" Buzones de Destino ");
        let loading_p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("⏳ Conectando con Microsoft Outlook y obteniendo buzones...", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("Por favor espere un momento mientras se leen las cuentas y almacenes MAPI.", Style::default().fg(Theme::TEXT_MUTED))),
        ])
        .alignment(Alignment::Center)
        .block(loading_block);
        f.render_widget(loading_p, chunks[1]);
    } else {
        let header_cells = ["Sel", "Buzón / Almacén", "Tipo de Almacén", "Tamaño en Disco", "Ruta de Archivo"]
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
                } else if item.store_type == "PST" {
                    Style::default().fg(Theme::WARNING)
                } else {
                    Style::default().fg(Theme::TEXT_MUTED)
                }
            );
            let size_cell = Cell::from(format!("[{}]", item.size_display));
            let path_cell = Cell::from(item.file_path.as_deref().unwrap_or(""));

            let row = Row::new(vec![sel_cell, name_cell, type_cell, size_cell, path_cell]);
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
                Constraint::Length(35), // Buzón / Almacén
                Constraint::Length(18), // Tipo de Almacén
                Constraint::Length(15), // Tamaño
                Constraint::Min(25),    // Ruta
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
                .title(" Buzones de Destino Disponibles (Marca uno o múltiples con Espacio) "),
        );

        f.render_widget(table, chunks[1]);
    }

    // 3. Advertencia si aplica
    let help_idx = if has_warning {
        if let Some(ref notice) = state.mailbox_warning_notice {
            let warn_p = Paragraph::new(Line::from(vec![
                Span::styled("⚠️  ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
                Span::styled(notice.as_str(), Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
            ]))
            .alignment(Alignment::Center);
            f.render_widget(warn_p, chunks[2]);
        }
        3
    } else {
        2
    };

    // 4. Atajos
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
    f.render_widget(Paragraph::new(help_line).alignment(Alignment::Center), chunks[help_idx]);
}
