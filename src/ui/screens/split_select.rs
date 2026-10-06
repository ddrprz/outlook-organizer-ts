use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Row, Table},
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
    let guide_lines = vec![
        Line::from(vec![
            Span::styled("SELECCIÓN DE ARCHIVO PST ORIGEN", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" — Selecciona el archivo que deseas dividir por periodos", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Usa ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[↑/↓]", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" para navegar, ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[Enter]", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" para seleccionar y avanzar, o ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[E]", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" para buscar con el explorador en otra carpeta.", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];
    let guide_p = Paragraph::new(guide_lines);
    f.render_widget(guide_p, chunks[0]);

    // 2. Tabla de PSTs
    let selected_idx = state.selected_pst_table_idx;
    let rows: Vec<Row> = if state.discovered_psts.is_empty() {
        vec![Row::new(vec![
            Span::styled("No se detectaron archivos .pst en la ruta actual.", Style::default().fg(Theme::WARNING)),
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
                let marker = if is_focused { " ▶ " } else { "   " };
                let size_str = format!("{:.2} MB", pst.size_mb);

                let has_cache = state.pst_details_cache.contains_key(&pst.path);
                let status_str = if has_cache {
                    "✓ Analizado"
                } else if state.inspecting_psts.contains(&pst.path) {
                    "⏳ Analizando..."
                } else {
                    "Listo para particionar"
                };

                let row_style = if is_focused {
                    Style::default().bg(Theme::SURFACE_HIGHLIGHT).fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_MAIN)
                };

                Row::new(vec![
                    Span::styled(format!("{}{}", marker, pst.name), row_style),
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
            Constraint::Length(32), // Nombre
            Constraint::Length(14), // Tamaño
            Constraint::Length(22), // Estado
            Constraint::Min(25),    // Ruta
        ],
    )
    .header(
        Row::new(vec![
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
    let selected_pst_info = state.discovered_psts.get(selected_idx);
    let info_text = if let Some(pst) = selected_pst_info {
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
                Span::styled("Archivo Seleccionado: ", Style::default().fg(Theme::TEXT_MUTED)),
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
        vec![Line::from(Span::styled("No hay ningún archivo PST seleccionado.", Style::default().fg(Theme::WARNING)))]
    };

    let info_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER_INACTIVE))
        .title(" Resumen de Selección ");
    let info_p = Paragraph::new(info_text).block(info_block);
    f.render_widget(info_p, chunks[2]);
}
