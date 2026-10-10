use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Row, Table},
    Frame,
};

use crate::{app::AppState, ui::theme::Theme};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Encabezado / Guía
            Constraint::Min(8),    // Tabla de PSTs disponibles
            Constraint::Length(4), // Panel de información / Atajos
        ])
        .split(area);

    // 1. Encabezado
    let selected_count = state.discovered_psts.iter().filter(|p| p.selected).count();
    let guide_lines = vec![
        Line::from(vec![
            Span::styled("SELECCIÓN DE ARCHIVOS PST ORIGEN", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" — Selecciona uno o varios archivos para dividir o consolidar", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
        Line::from(vec![
            Span::styled("Archivos detectados: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!("{}", state.discovered_psts.len()), Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
            Span::styled(" | Marcados para procesar: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", selected_count),
                Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("[↑/↓] Navegar  •  [Espacio] Marcar/Desmarcar  •  [A] Todos  •  [D] Ninguno  •  [Enter] Continuar  •  [E] Explorador", Style::default().fg(Theme::ACCENT_PRIMARY)),
        ]),
    ];
    let guide_p = Paragraph::new(guide_lines);
    f.render_widget(guide_p, chunks[0]);

    // 2. Tabla de PSTs
    let selected_idx = state.selected_pst_table_idx;
    let rows: Vec<Row> = if state.discovered_psts.is_empty() {
        vec![Row::new(vec![
            Span::raw(""),
            Span::styled("No se detectaron archivos .pst en la ruta actual.", Style::default().fg(Theme::WARNING)),
            Span::raw(""),
            Span::raw(""),
            Span::raw(""),
        ])]
    } else {
        state
            .discovered_psts
            .iter()
            .enumerate()
            .map(|(i, pst)| {
                let is_focused = i == selected_idx;
                let checkbox = if pst.selected { "[x]" } else { "[ ]" };
                let size_str = format!("{:.2} MB", pst.size_mb);

                let has_cache = state.pst_details_cache.contains_key(&pst.path);
                let status_str = if has_cache {
                    "✓ Analizado"
                } else if state.inspecting_psts.contains(&pst.path) {
                    "⧗ Analizando..."
                } else {
                    "Listo para particionar"
                };

                let row_style = if is_focused {
                    Style::default().bg(Theme::SURFACE_HIGHLIGHT).fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_MAIN)
                };

                let cb_style = if pst.selected {
                    Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_MUTED)
                };

                Row::new(vec![
                    Span::styled(format!(" {}", checkbox), cb_style),
                    Span::styled(pst.name.clone(), row_style),
                    Span::styled(size_str, if is_focused { Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
                    Span::styled(status_str, if has_cache { Style::default().fg(Theme::SUCCESS) } else { Style::default().fg(Theme::TEXT_MUTED) }),
                    Span::styled(pst.path.clone(), Style::default().fg(Theme::TEXT_MUTED)),
                ])
            })
            .collect()
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(6),  // [x]
            Constraint::Length(32), // Nombre
            Constraint::Length(14), // Tamaño
            Constraint::Length(22), // Estado
            Constraint::Min(25),    // Ruta
        ],
    )
    .header(
        Row::new(vec![
            Span::styled(" Sel", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" Nombre de Archivo", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" Tamaño", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" Inspección", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" Ruta en Disco", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        ])
        .style(Style::default().bg(Theme::SURFACE)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
            .title(format!(" Archivos PST Disponibles ({}) ", state.discovered_psts.len())),
    );

    f.render_widget(table, chunks[1]);

    // 3. Panel de Información
    let info_text = if selected_count > 1 {
        let total_size: f64 = state.discovered_psts.iter().filter(|p| p.selected).map(|p| p.size_mb).sum();
        let selected_names = state.discovered_psts.iter().filter(|p| p.selected).map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ");
        vec![
            Line::from(vec![
                Span::styled("Múltiples archivos seleccionados: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(format!("{} archivos ({:.2} MB en total)", selected_count, total_size), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("PSTs: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(selected_names, Style::default().fg(Theme::TEXT_MAIN)),
            ]),
        ]
    } else if let Some(pst) = state.discovered_psts.get(selected_idx) {
        let details_info = if let Some(det) = state.pst_details_cache.get(&pst.path) {
            format!(
                " • Años detectados: {:?} • Total correos: {}",
                det.years, det.total_items
            )
        } else {
            " • Presiona Enter para cargar y filtrar periodos".to_string()
        };
        vec![
            Line::from(vec![
                Span::styled("Archivo Enfocado: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(&pst.name, Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" ({:.2} MB)", pst.size_mb), Style::default().fg(Theme::TEXT_MAIN)),
                Span::styled(details_info, Style::default().fg(Theme::SUCCESS)),
            ]),
            Line::from(vec![
                Span::styled("Ruta: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(&pst.path, Style::default().fg(Theme::TEXT_MAIN)),
            ]),
        ]
    } else {
        vec![Line::from(Span::styled("No hay ningún archivo PST disponible.", Style::default().fg(Theme::WARNING)))]
    };

    let info_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER_INACTIVE))
        .title(" Resumen de Selección ");
    let info_p = Paragraph::new(info_text).block(info_block);
    f.render_widget(info_p, chunks[2]);

    // Modal flotante si el usuario pulsó Enter y el PST aún se está escaneando
    if state.split.waiting_to_advance {
        let popup_width = 68.min(area.width.saturating_sub(4));
        let popup_height = 9.min(area.height.saturating_sub(4));
        let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
        let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(x, y, popup_width, popup_height);

        f.render_widget(Clear, popup_area);

        let pst_name = state.split.source_pst.as_ref().map(|p| p.name.as_str()).unwrap_or("PST");
        let folder_info = if let Some(ref f) = state.split.scanning_folder {
            format!("Carpeta en análisis: {}", f)
        } else {
            "Conectando a almacén MAPI...".to_string()
        };

        let modal_lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Archivo: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(pst_name, Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled(format!("  •  Correos leídos: {}", state.split.scanned_items), Style::default().fg(Theme::TEXT_MAIN)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ⧗ ", Style::default().fg(Theme::BRAND_PRIMARY)),
                Span::styled(folder_info, Style::default().fg(Theme::TEXT_MUTED)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Descubriendo años y meses exactos... ", Style::default().fg(Theme::ACCENT_PRIMARY)),
                Span::styled("[Esc para cancelar]", Style::default().fg(Theme::TEXT_MUTED)),
            ]),
        ];

        let modal_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
            .title(" ◈ Escaneando Estructura y Periodos del Archivo PST ");
        f.render_widget(Paragraph::new(modal_lines).block(modal_block), popup_area);
    }
}
