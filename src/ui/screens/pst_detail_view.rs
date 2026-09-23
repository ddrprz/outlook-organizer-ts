use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::{
    app::{AppState, PstDetail, PstDetailModalState, PstFolderItemType},
    ui::theme::Theme,
};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    match &state.pst_detail_modal {
        PstDetailModalState::Loading { pst_path, pst_name } => {
            render_loading(f, area, pst_name, pst_path);
        }
        PstDetailModalState::Error { pst_name, message } => {
            render_error(f, area, pst_name, message);
        }
        PstDetailModalState::Loaded(detail) => {
            render_loaded_view(f, area, state, detail);
        }
        PstDetailModalState::Closed => {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Detalle de PST ");
            f.render_widget(Paragraph::new("No hay información de PST cargada. Presione [Esc] para volver."), block.inner(area));
            f.render_widget(block, area);
        }
    }
}

fn render_loading(f: &mut Frame, area: Rect, pst_name: &str, pst_path: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Theme::BRAND_PRIMARY))
        .title(format!(" ◈ Inspeccionando PST: {} ", pst_name));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⏳ ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(
                "Analizando estructura interna MAPI, carpetas y métricas temporales...",
                Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Archivo: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(pst_path, Style::default().fg(Theme::ACCENT_PRIMARY)),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Extrayendo jerarquía de carpetas, fechas, distribución por años y meses.", Style::default().fg(Theme::TEXT_MUTED))),
        Line::from(Span::styled("  Calculando tamaños por periodo y volumen de mensajes...", Style::default().fg(Theme::SUCCESS))),
        Line::from(""),
        Line::from(Span::styled("  [Esc / Q] Cancelar y volver", Style::default().fg(Theme::TEXT_MUTED))),
    ];

    f.render_widget(Paragraph::new(content), inner);
}

fn render_error(f: &mut Frame, area: Rect, pst_name: &str, message: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Theme::DANGER))
        .title(format!(" ⚠ Error al Inspeccionar: {} ", pst_name));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let content = vec![
        Line::from(""),
        Line::from(Span::styled("  No se pudo leer la estructura interna del archivo PST:", Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled(format!("  {}", message), Style::default().fg(Theme::TEXT_MAIN))),
        Line::from(""),
        Line::from(Span::styled("  [Esc / Enter / Q] Volver al listado", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD))),
    ];

    f.render_widget(Paragraph::new(content), inner);
}

fn render_loaded_view(f: &mut Frame, area: Rect, state: &AppState, detail: &PstDetail) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Header con resumen del PST
            Constraint::Min(10),   // Dos columnas
        ])
        .split(area);

    // 1. Header Superior
    render_header_card(f, main_chunks[0], detail);

    // 2. Dos columnas (Izquierda: Explorador de Carpetas 42%, Derecha: Métricas Dinámicas 58%)
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(42),
            Constraint::Percentage(58),
        ])
        .split(main_chunks[1]);

    // Columna 1: Explorador de Carpetas
    render_folder_explorer(f, body_chunks[0], state, detail);

    // Columna 2: Métricas dinámicas (Carpeta o Global)
    render_metrics_panel(f, body_chunks[1], state, detail);
}

fn render_header_card(f: &mut Frame, area: Rect, detail: &PstDetail) {
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(format!(" ◈ Detalle de Archivo PST: {} ", detail.file_name));

    let inner = header_block.inner(area);
    f.render_widget(header_block, area);

    let first_date = detail.first_email_date.as_deref().unwrap_or("N/D");
    let last_date = detail.last_email_date.as_deref().unwrap_or("N/D");

    let lines = vec![
        Line::from(vec![
            Span::styled("Ruta: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!("📁 {} ", detail.file_path), Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" | Peso Total: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!("{:.1} MB", detail.size_mb), Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
            Span::styled(" | Total de Correos: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!("{} correos", detail.total_items), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Rango de Fechas Detectado: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!("Desde: {}  ➜  Hasta: {}", first_date, last_date), Style::default().fg(Theme::TEXT_MAIN)),
            Span::styled(format!(" | Total Carpetas: {}", detail.folders.len()), Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];

    f.render_widget(Paragraph::new(lines), inner);
}

fn render_folder_explorer(f: &mut Frame, area: Rect, state: &AppState, detail: &PstDetail) {
    let explorer = &state.pst_folder_explorer;
    let entries = explorer.current_entries(detail);

    let parent_path_display = explorer
        .current_parent
        .as_deref()
        .map(|p| format!(" [Ruta: \\{}]", p))
        .unwrap_or_else(|| " [Raíz del PST]".to_string());

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
        .title(format!(" 📁 Explorador de Carpetas{} ", parent_path_display));

    let header_cells = ["Sel", "Carpeta / Directorio", "Correos", "Tamaño"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = entries.iter().enumerate().map(|(idx, entry)| {
        let is_cursor = idx == explorer.selected_idx;
        let is_checked = explorer.checked_folder_path.as_deref() == Some(&entry.path);

        let sel_cell = match entry.item_type {
            PstFolderItemType::ParentDir => Cell::from(" "),
            PstFolderItemType::Folder => {
                if is_checked {
                    Cell::from("[x]").style(Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD))
                } else {
                    Cell::from("[ ]").style(Style::default().fg(Theme::TEXT_MUTED))
                }
            }
        };

        let name_text = match entry.item_type {
            PstFolderItemType::ParentDir => ".. (Subir de nivel)".to_string(),
            PstFolderItemType::Folder => {
                if entry.has_children {
                    format!("📁 {} ▶", entry.name)
                } else {
                    format!("📁 {}", entry.name)
                }
            }
        };
        let name_cell = Cell::from(name_text);

        let count_text = match entry.item_type {
            PstFolderItemType::ParentDir => "-".to_string(),
            PstFolderItemType::Folder => format!("{}", entry.count),
        };
        let count_cell = Cell::from(count_text);

        let size_text = match entry.item_type {
            PstFolderItemType::ParentDir => "-".to_string(),
            PstFolderItemType::Folder => {
                if entry.size_mb > 0.0 {
                    format!("{:.1} MB", entry.size_mb)
                } else {
                    "0 MB".to_string()
                }
            }
        };
        let size_cell = Cell::from(size_text);

        let row = Row::new(vec![sel_cell, name_cell, count_cell, size_cell]);
        if is_cursor {
            row.style(Style::default().bg(Theme::BG_CARD).fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD))
        } else if is_checked {
            row.style(Style::default().fg(Theme::SUCCESS))
        } else {
            row
        }
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Min(16),
            Constraint::Length(10),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(block);

    f.render_widget(table, area);
}

fn render_metrics_panel(f: &mut Frame, area: Rect, state: &AppState, detail: &PstDetail) {
    let explorer = &state.pst_folder_explorer;
    let selected_folder = explorer.get_selected_folder_stats(detail);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Resumen estadístico
            Constraint::Min(6),    // Tablas de años y meses
        ])
        .split(area);

    if let Some(folder) = selected_folder {
        // --- CASO 1: Carpeta Seleccionada con Espacio ---
        let block_title = format!(" 📂 Detalle de Carpeta: {} ", folder.name);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::SUCCESS))
            .title(block_title);

        let inner = block.inner(chunks[0]);
        f.render_widget(block, chunks[0]);

        let years_str = if folder.years.is_empty() {
            "Sin correos".to_string()
        } else {
            folder.years.iter().map(|y| y.to_string()).collect::<Vec<_>>().join(", ")
        };

        let month_count: usize = folder.year_months.values().map(|m| m.len()).sum();

        let lines = vec![
            Line::from(vec![
                Span::styled("Carpeta: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(&folder.name, Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" (Ruta: \\{})", if folder.path.is_empty() { &folder.name } else { &folder.path }), Style::default().fg(Theme::TEXT_MUTED)),
            ]),
            Line::from(vec![
                Span::styled("Total Correos: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(format!("{} ", folder.count), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled("| Tamaño Estimado: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(format!("{:.2} MB ", folder.size_mb), Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
                Span::styled(format!("| Meses con Actividad: {}", month_count), Style::default().fg(Theme::TEXT_MUTED)),
            ]),
            Line::from(vec![
                Span::styled("Años Presentes: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(years_str, Style::default().fg(Theme::ACCENT_PRIMARY)),
                Span::styled("  [Espacio para deseleccionar y ver PST completo]", Style::default().fg(Theme::TEXT_MUTED)),
            ]),
        ];
        f.render_widget(Paragraph::new(lines), inner);

        // Tablas de desglose para la carpeta
        render_breakdown_tables(
            f,
            chunks[1],
            &folder.counts_by_year,
            &folder.sizes_by_year_mb,
            &folder.counts_by_month,
            &folder.sizes_by_month_mb,
            folder.count,
        );
    } else {
        // --- CASO 2: Resumen Global del Archivo PST ---
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::BRAND_PRIMARY))
            .title(format!(" 🌐 Métricas Consolidadas del PST: {} ", detail.file_name));

        let inner = block.inner(chunks[0]);
        f.render_widget(block, chunks[0]);

        let years_str = if detail.years.is_empty() {
            "Sin correos".to_string()
        } else {
            detail.years.iter().map(|y| y.to_string()).collect::<Vec<_>>().join(", ")
        };

        let month_count: usize = detail.year_months.values().map(|m| m.len()).sum();

        let lines = vec![
            Line::from(vec![
                Span::styled("Estado: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled("Vista Global del PST (Ninguna carpeta seleccionada)", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Total Correos PST: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(format!("{} ", detail.total_items), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled("| Peso en Disco: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(format!("{:.1} MB ", detail.size_mb), Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
                Span::styled(format!("| Total de Meses: {}", month_count), Style::default().fg(Theme::TEXT_MUTED)),
            ]),
            Line::from(vec![
                Span::styled("Años Detectados: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(years_str, Style::default().fg(Theme::ACCENT_PRIMARY)),
                Span::styled("  [Presione Espacio sobre una carpeta para aislar sus métricas]", Style::default().fg(Theme::TEXT_MUTED)),
            ]),
        ];
        f.render_widget(Paragraph::new(lines), inner);

        // Tablas de desglose globales
        render_breakdown_tables(
            f,
            chunks[1],
            &detail.counts_by_year,
            &detail.sizes_by_year_mb,
            &detail.counts_by_month,
            &detail.sizes_by_month_mb,
            detail.total_items,
        );
    }
}

fn render_breakdown_tables(
    f: &mut Frame,
    area: Rect,
    counts_by_year: &std::collections::BTreeMap<String, usize>,
    sizes_by_year: &std::collections::BTreeMap<String, f64>,
    counts_by_month: &std::collections::BTreeMap<String, usize>,
    sizes_by_month: &std::collections::BTreeMap<String, f64>,
    total_items: usize,
) {
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45), // Distribución por Año
            Constraint::Percentage(55), // Distribución por Mes
        ])
        .split(area);

    // 1. Tabla de Años
    let year_header_cells = ["Año", "Correos", "Tamaño", "% Total"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)));
    let year_header = Row::new(year_header_cells).height(1).bottom_margin(1);

    let year_rows = counts_by_year.iter().rev().map(|(year, count)| {
        let size_mb = sizes_by_year.get(year).copied().unwrap_or(0.0);
        let pct = if total_items > 0 {
            (*count as f64 / total_items as f64) * 100.0
        } else {
            0.0
        };

        Row::new(vec![
            Cell::from(year.as_str()).style(Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Cell::from(format!("{}", count)).style(Style::default().fg(Theme::TEXT_MAIN)),
            Cell::from(format!("{:.1} MB", size_mb)).style(Style::default().fg(Theme::SUCCESS)),
            Cell::from(format!("{:.1}%", pct)).style(Style::default().fg(Theme::BRAND_PRIMARY)),
        ])
    });

    let year_table = Table::new(
        year_rows,
        [
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(11),
            Constraint::Min(8),
        ],
    )
    .header(year_header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
            .title(" Distribución por Año "),
    );
    f.render_widget(year_table, split[0]);

    // 2. Tabla de Meses
    let month_header_cells = ["Mes", "Correos", "Tamaño (MB)"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Theme::ACCENT_SECONDARY).add_modifier(Modifier::BOLD)));
    let month_header = Row::new(month_header_cells).height(1).bottom_margin(1);

    let month_rows = counts_by_month.iter().rev().map(|(month, count)| {
        let size_mb = sizes_by_month.get(month).copied().unwrap_or(0.0);
        Row::new(vec![
            Cell::from(month.as_str()).style(Style::default().fg(Theme::TEXT_MAIN)),
            Cell::from(format!("{}", count)).style(Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Cell::from(format!("{:.2} MB", size_mb)).style(Style::default().fg(Theme::SUCCESS)),
        ])
    });

    let month_table = Table::new(
        month_rows,
        [
            Constraint::Length(12),
            Constraint::Length(11),
            Constraint::Min(12),
        ],
    )
    .header(month_header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
            .title(" Distribución por Mes (Recientes) "),
    );
    f.render_widget(month_table, split[1]);
}
