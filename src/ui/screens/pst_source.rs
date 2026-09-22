use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::{app::AppState, ui::theme::Theme};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // Modo de escaneo y ruta
            Constraint::Min(8),     // Tabla de PSTs encontrados
            Constraint::Length(2),  // Atajos rápidos
        ])
        .split(area);

    // 1. Selector de Origen / Ruta
    let source_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Ubicación de Búsqueda ");

    let source_inner = source_block.inner(chunks[0]);
    f.render_widget(source_block, chunks[0]);

    let source_lines = vec![
        Line::from(vec![
            Span::styled("Ruta actual de escaneo: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!("📁 {}", state.pst_scan_path), Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" (Ruta por defecto C:\\Correo)", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
        Line::from(vec![
            Span::styled("Archivos detectados: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!("{}", state.discovered_psts.len()), Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
            Span::styled(" | Marcados para importar: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", state.discovered_psts.iter().filter(|p| p.selected).count()),
                Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(source_lines), source_inner);

    // 2. Tabla de PSTs interactiva
    let header_cells = ["Sel", "Nombre del Archivo", "Tamaño (MB)", "Ruta Completa"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = state.discovered_psts.iter().enumerate().map(|(idx, item)| {
        let is_focused = idx == state.selected_pst_table_idx;
        let checkbox = if item.selected { "[x]" } else { "[ ]" };
        let sel_cell = Cell::from(checkbox).style(if item.selected {
            Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_MUTED)
        });

        let name_cell = Cell::from(item.name.as_str());
        let size_cell = Cell::from(format!("{:.1} MB", item.size_mb));
        let path_cell = Cell::from(item.path.as_str());

        let row = Row::new(vec![sel_cell, name_cell, size_cell, path_cell]);
        if is_focused {
            row.style(Style::default().bg(Theme::BG_CARD).fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD))
        } else {
            row
        }
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Length(25),
            Constraint::Length(15),
            Constraint::Min(30),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
            .title(" Archivos PST Seleccionados "),
    );

    f.render_widget(table, chunks[1]);

    // 3. Atajos
    let help_line = Line::from(vec![
        Span::styled("[↑/↓] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Navegar fila   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[Espacio] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Marcar/Desmarcar   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[D] ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Ver Detalle PST   ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("[E] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Explorar   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[A/N] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Todos/Ninguno   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[Enter] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        Span::styled("Continuar", Style::default().fg(Theme::SUCCESS)),
    ]);
    f.render_widget(Paragraph::new(help_line).alignment(Alignment::Center), chunks[2]);

    // 4. Modal Flotante de Detalle PST (si está activo)
    if state.pst_detail_modal != crate::app::PstDetailModalState::Closed {
        render_pst_detail_modal(f, area, state);
    }
}

pub fn render_pst_detail_modal(f: &mut Frame, area: Rect, state: &AppState) {
    use ratatui::widgets::Clear;

    let modal_width = 86.min(area.width.saturating_sub(4));
    let modal_height = 20.min(area.height.saturating_sub(2));

    let x = area.x + (area.width.saturating_sub(modal_width)) / 2;
    let y = area.y + (area.height.saturating_sub(modal_height)) / 2;
    let modal_rect = Rect::new(x, y, modal_width, modal_height);

    f.render_widget(Clear, modal_rect);

    match &state.pst_detail_modal {
        crate::app::PstDetailModalState::Closed => {}
        crate::app::PstDetailModalState::Loading { pst_name, pst_path } => {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Theme::BRAND_PRIMARY))
                .title(format!(" ◈ Inspeccionando PST: {} ", pst_name));

            let inner = block.inner(modal_rect);
            f.render_widget(block, modal_rect);

            let content = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  ⏳ ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                    Span::styled("Analizando estructura interna MAPI del archivo PST...", Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Archivo: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(pst_path, Style::default().fg(Theme::ACCENT_PRIMARY)),
                ]),
                Line::from(""),
                Line::from(Span::styled("  Extrayendo carpetas, fechas, años/meses y el correo más reciente.", Style::default().fg(Theme::TEXT_MUTED))),
                Line::from(Span::styled("  Por favor espere unos segundos...", Style::default().fg(Theme::SUCCESS))),
                Line::from(""),
                Line::from(Span::styled("  [Esc / Q] Cerrar ventana", Style::default().fg(Theme::TEXT_MUTED))),
            ];

            f.render_widget(Paragraph::new(content), inner);
        }
        crate::app::PstDetailModalState::Error { pst_name, message } => {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Theme::DANGER))
                .title(format!(" ⚠ Error al Inspeccionar: {} ", pst_name));

            let inner = block.inner(modal_rect);
            f.render_widget(block, modal_rect);

            let content = vec![
                Line::from(""),
                Line::from(Span::styled("  No se pudo leer la estructura interna del archivo PST:", Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(Span::styled(format!("  {}", message), Style::default().fg(Theme::TEXT_MAIN))),
                Line::from(""),
                Line::from(Span::styled("  [Esc / Enter / D] Cerrar", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD))),
            ];

            f.render_widget(Paragraph::new(content), inner);
        }
        crate::app::PstDetailModalState::Loaded(detail) => {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
                .title(format!(" ◈ Detalle Analítico del PST: {} ", detail.file_name));

            let inner = block.inner(modal_rect);
            f.render_widget(block, modal_rect);

            let last_email_display = detail
                .last_email_date
                .as_deref()
                .unwrap_or("No detectado / Sin correos");

            let first_email_display = detail
                .first_email_date
                .as_deref()
                .unwrap_or("No detectado");

            let month_short_names = ["Ene", "Feb", "Mar", "Abr", "May", "Jun", "Jul", "Ago", "Sep", "Oct", "Nov", "Dic"];

            let mut lines = vec![
                Line::from(vec![
                    Span::styled("📁 Ruta:   ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(&detail.file_path, Style::default().fg(Theme::TEXT_MAIN)),
                ]),
                Line::from(vec![
                    Span::styled("💾 Tamaño: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(format!("{:.1} MB ({:.2} GB)", detail.size_mb, detail.size_mb / 1024.0), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                    Span::styled("   |   Total de Correos: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(format!("{}", detail.total_items), Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("📅 ÚLTIMO CORREO (Más Reciente): ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                    Span::styled(last_email_display, Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled("📅 Primer Correo (Más Antiguo):  ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(first_email_display, Style::default().fg(Theme::TEXT_MAIN)),
                ]),
                Line::from(""),
                Line::from(Span::styled("🗓 AÑOS Y MESES CONTENIDOS EN EL PST:", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD))),
            ];

            if detail.years.is_empty() {
                lines.push(Line::from(Span::styled("  (No se detectaron correos con fecha válida en el archivo)", Style::default().fg(Theme::TEXT_MUTED))));
            } else {
                for y in &detail.years {
                    let y_str = format!("{}", y);
                    let months_span = if let Some(months) = detail.year_months.get(&y_str) {
                        let m_strs: Vec<String> = months.iter().map(|m| {
                            let idx = (*m as usize).saturating_sub(1);
                            month_short_names.get(idx).unwrap_or(&"?").to_string()
                        }).collect();
                        format!("{} ({} meses)", m_strs.join(", "), months.len())
                    } else {
                        "Todos los meses".to_string()
                    };

                    lines.push(Line::from(vec![
                        Span::styled(format!("  • Año {}: ", y), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                        Span::styled(months_span, Style::default().fg(Theme::TEXT_MAIN)),
                    ]));
                }
            }

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("📂 DESGLOSE POR CARPETAS:", Style::default().fg(Theme::ACCENT_SECONDARY).add_modifier(Modifier::BOLD))));

            if detail.folders.is_empty() {
                lines.push(Line::from(Span::styled("  (Sin carpetas con correos)", Style::default().fg(Theme::TEXT_MUTED))));
            } else {
                let folder_strs: Vec<String> = detail.folders.iter().map(|f| format!("{}: {}", f.name, f.count)).collect();
                lines.push(Line::from(Span::styled(format!("  {}", folder_strs.join("  |  ")), Style::default().fg(Theme::TEXT_MAIN))));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("[Esc / Enter / D] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled("Cerrar ventana de detalle", Style::default().fg(Theme::TEXT_MUTED)),
            ]));

            f.render_widget(Paragraph::new(lines), inner);
        }
    }
}
