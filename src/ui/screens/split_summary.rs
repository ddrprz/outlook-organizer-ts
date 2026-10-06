use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{AppState, SplitTransferMode},
    ui::theme::Theme,
};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Guía superior
            Constraint::Min(12),   // Cuerpo (Resumen de parámetros + Previsualización de archivos)
            Constraint::Length(4), // Botones de confirmación
        ])
        .split(area);

    // 1. Guía superior
    let guide_lines = vec![
        Line::from(vec![
            Span::styled("RESUMEN PRE-VUELO Y CONFIRMACIÓN", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" — Revisa los parámetros antes de iniciar la creación de los archivos", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
        Line::from(vec![
            Span::styled("Presiona ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[Enter]", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
            Span::styled(" para iniciar el proceso de partición, o ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[Esc]", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
            Span::styled(" para retroceder y modificar la configuración.", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];
    f.render_widget(Paragraph::new(guide_lines), chunks[0]);

    // 2. Cuerpo en dos columnas
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Columna Parámetros
            Constraint::Percentage(50), // Columna Archivos a Generar
        ])
        .split(chunks[1]);

    // COLUMNA IZQUIERDA: PARÁMETROS
    let (source_pst_label, source_pst_name, source_pst_size) = if state.split.source_psts.len() > 1 {
        let total_size: f64 = state.split.source_psts.iter().map(|p| p.size_mb).sum();
        (
            "• Archivos PST Origen: ",
            format!("{} archivos seleccionados", state.split.source_psts.len()),
            format!("{:.2} MB en total", total_size),
        )
    } else {
        let name = state
            .split
            .source_pst
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "No seleccionado".to_string());
        let size = state
            .split
            .source_pst
            .as_ref()
            .map(|p| format!("{:.2} MB", p.size_mb))
            .unwrap_or_else(|| "0 MB".to_string());
        ("• Archivo PST Origen:  ", name, size)
    };

    let years_str = if state.split.selected_years.is_empty() {
        "Todos los años disponibles".to_string()
    } else {
        let v: Vec<String> = state.split.selected_years.iter().map(|y| y.to_string()).collect();
        v.join(", ")
    };

    let months_str = if state.split.selected_months.len() == 12 || state.split.selected_months.is_empty() {
        "Todos los meses (Año completo)".to_string()
    } else {
        format!("{} meses seleccionados", state.split.selected_months.len())
    };

    let param_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(source_pst_label, Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(source_pst_name, Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" ({})", source_pst_size), Style::default().fg(Theme::TEXT_MAIN)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("• Años a extraer:      ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(years_str, Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("• Meses a extraer:     ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(months_str, Style::default().fg(Theme::TEXT_MAIN)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("• Modo de partición:   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(state.split.partition_mode.label(), Style::default().fg(Theme::ACCENT_PRIMARY)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("• Modo de acción:      ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                state.split.transfer_mode.label(),
                if state.split.transfer_mode == SplitTransferMode::Move {
                    Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)
                },
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("• Directorio destino:  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(&state.split.output_dir, Style::default().fg(Theme::TEXT_MAIN)),
        ]),
    ];

    let param_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER_INACTIVE))
        .title(" Parámetros de la Operación ");
    f.render_widget(Paragraph::new(param_lines).block(param_block), body_chunks[0]);

    // COLUMNA DERECHA: ARCHIVOS PREVISTOS A GENERAR
    let predicted_files = state.preview_split_filenames();
    let mut files_lines = Vec::new();
    files_lines.push(Line::from(""));
    files_lines.push(Line::from(Span::styled(
        format!("Se generarán {} archivo(s) PST en destino:", predicted_files.len()),
        Style::default().fg(Theme::TEXT_MUTED),
    )));
    files_lines.push(Line::from(""));

    for (i, file_name) in predicted_files.iter().enumerate() {
        if i >= 10 {
            files_lines.push(Line::from(Span::styled(
                format!("  ... y {} archivo(s) adicionales", predicted_files.len() - 10),
                Style::default().fg(Theme::TEXT_MUTED),
            )));
            break;
        }
        files_lines.push(Line::from(vec![
            Span::styled(format!("  {:2}. ", i + 1), Style::default().fg(Theme::ACCENT_PRIMARY)),
            Span::styled(file_name, Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("  [Nuevo PST Unicode]", Style::default().fg(Theme::TEXT_MUTED)),
        ]));
    }

    let files_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(format!(" Vista Previa de Salida ({}) ", predicted_files.len()));
    f.render_widget(Paragraph::new(files_lines).block(files_block), body_chunks[1]);

    // 3. Botones de confirmación
    let btn_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  [ ENTER: INICIAR SEPARACIÓN DE PSTs ]  ", Style::default().bg(Theme::SUCCESS).fg(Theme::BG_DARK).add_modifier(Modifier::BOLD)),
            Span::raw("    "),
            Span::styled("  [ ESC: RETROCEDER ]  ", Style::default().bg(Theme::SURFACE).fg(Theme::TEXT_MUTED)),
        ]),
    ];
    f.render_widget(Paragraph::new(btn_lines).alignment(Alignment::Center), chunks[2]);
}
