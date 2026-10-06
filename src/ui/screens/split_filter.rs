use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
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
            Constraint::Length(3), // Guía superior
            Constraint::Min(12),   // Paneles de Años y Meses
            Constraint::Length(5), // Carpetas a incluir
        ])
        .split(area);

    // 1. Guía superior
    let guide_lines = vec![
        Line::from(vec![
            Span::styled("FILTRO DE PERIODOS Y CARPETAS", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" — Selecciona los años y meses que deseas extraer hacia nuevos PSTs", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
        Line::from(vec![
            Span::styled("[Tab] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Alternar entre Años y Meses  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[Espacio] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Marcar/Desmarcar  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[A/N] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Todos/Ninguno  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[1/2] ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Semestres  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[Enter] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
            Span::styled("Siguiente", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];
    f.render_widget(Paragraph::new(guide_lines), chunks[0]);

    // 2. Paneles de Años y Meses (Split horizontal)
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Años
            Constraint::Percentage(50), // Meses
        ])
        .split(chunks[1]);

    // Detalle en caché del PST de origen
    let pst_detail = state
        .split
        .source_pst
        .as_ref()
        .and_then(|p| state.pst_details_cache.get(&p.path));

    // PANEL IZQUIERDO: AÑOS
    let mut year_lines = Vec::new();
    year_lines.push(Line::from(""));

    let available_years = &state.split.available_years;
    for (i, &y) in available_years.iter().enumerate() {
        let is_cursor = state.split.year_cursor == i;
        let is_selected = state.split.selected_years.contains(&y);

        let check_box = if is_selected { "[x] " } else { "[ ] " };
        let count_str = if let Some(det) = pst_detail {
            det.counts_by_year
                .get(&y.to_string())
                .map(|cnt| format!(" ({} correos)", cnt))
                .unwrap_or_else(|| " (0 correos)".to_string())
        } else {
            String::new()
        };

        let pointer = if is_cursor { "▶ " } else { "  " };

        let line_style = if is_cursor {
            Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default().fg(Theme::TEXT_MAIN)
        } else {
            Style::default().fg(Theme::TEXT_MUTED)
        };

        year_lines.push(Line::from(vec![
            Span::styled(pointer, Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(check_box, if is_selected { Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
            Span::styled(format!("Año {}", y), line_style),
            Span::styled(count_str, Style::default().fg(Theme::TEXT_MUTED)),
        ]));
    }

    if available_years.is_empty() {
        year_lines.push(Line::from(Span::styled("No hay años detectados. Se procesará todo el archivo.", Style::default().fg(Theme::WARNING))));
    }

    let years_count_selected = state.split.selected_years.len();
    let years_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if state.split.config_cursor == 0 { Theme::ACCENT_PRIMARY } else { Theme::BORDER_INACTIVE }))
        .title(format!(" Años Seleccionados ({}/{}) [A: Todos / N: Ninguno] ", years_count_selected, available_years.len()));
    f.render_widget(Paragraph::new(year_lines).block(years_block), body_chunks[0]);

    // PANEL DERECHO: MESES (Solo los meses que realmente existen en el PST para los años seleccionados)
    let mut month_lines = Vec::new();
    month_lines.push(Line::from(""));

    let available_months = &state.split.available_months;
    let target_years: Vec<String> = if state.split.selected_years.is_empty() {
        state.split.available_years.iter().map(|y| y.to_string()).collect()
    } else {
        state.split.selected_years.iter().map(|y| y.to_string()).collect()
    };

    for (i, &m_num) in available_months.iter().enumerate() {
        let is_cursor = state.split.month_cursor == i;
        let is_selected = state.split.selected_months.contains(&m_num);

        let check_box = if is_selected { "[x] " } else { "[ ] " };
        let pointer = if is_cursor { "▶ " } else { "  " };

        let m_name = match m_num {
            1 => "01 - Enero",
            2 => "02 - Febrero",
            3 => "03 - Marzo",
            4 => "04 - Abril",
            5 => "05 - Mayo",
            6 => "06 - Junio",
            7 => "07 - Julio",
            8 => "08 - Agosto",
            9 => "09 - Septiembre",
            10 => "10 - Octubre",
            11 => "11 - Noviembre",
            12 => "12 - Diciembre",
            _ => "Mes desconocido",
        };

        let count_str = if let Some(det) = pst_detail {
            let cnt: usize = target_years
                .iter()
                .filter_map(|y| {
                    let ym_key = format!("{}-{:02}", y, m_num);
                    det.counts_by_month.get(&ym_key).copied()
                })
                .sum();
            if cnt > 0 {
                format!(" ({} correos)", cnt)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let line_style = if is_cursor {
            Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default().fg(Theme::TEXT_MAIN)
        } else {
            Style::default().fg(Theme::TEXT_MUTED)
        };

        month_lines.push(Line::from(vec![
            Span::styled(pointer, Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(check_box, if is_selected { Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
            Span::styled(m_name, line_style),
            Span::styled(count_str, Style::default().fg(Theme::TEXT_MUTED)),
        ]));
    }

    if available_months.is_empty() {
        month_lines.push(Line::from(Span::styled("No hay meses con correos para los años seleccionados.", Style::default().fg(Theme::WARNING))));
    }

    let months_count_selected = state.split.selected_months.len();
    let months_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if state.split.config_cursor == 1 { Theme::ACCENT_PRIMARY } else { Theme::BORDER_INACTIVE }))
        .title(format!(" Meses Seleccionados ({}/{}) [1: Sem 1 / 2: Sem 2 / T: Todos] ", months_count_selected, available_months.len()));
    f.render_widget(Paragraph::new(month_lines).block(months_block), body_chunks[1]);

    // 3. Panel Inferior: Carpetas a Incluir
    let folders_line = vec![
        Span::styled("Carpetas de origen a extraer:  ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(if state.split.include_inbox { "[x] " } else { "[ ] " }, if state.split.include_inbox { Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
        Span::styled("Bandeja de Entrada [I]    ", Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled(if state.split.include_sent { "[x] " } else { "[ ] " }, if state.split.include_sent { Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
        Span::styled("Elementos Enviados [S]    ", Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled(if state.split.include_deleted { "[x] " } else { "[ ] " }, if state.split.include_deleted { Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
        Span::styled("Elementos Eliminados [D]    ", Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled(if state.split.include_custom_folders { "[x] " } else { "[ ] " }, if state.split.include_custom_folders { Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
        Span::styled("Carpetas Personalizadas [C]", Style::default().fg(Theme::TEXT_MAIN)),
    ];

    let folders_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER_INACTIVE))
        .title(" Alcance de Carpetas ");
    let folders_p = Paragraph::new(vec![Line::from(""), Line::from(folders_line)]).block(folders_block);
    f.render_widget(folders_p, chunks[2]);
}
