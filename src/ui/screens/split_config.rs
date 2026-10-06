use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{AppState, SplitPartitionMode, SplitTransferMode},
    ui::theme::Theme,
};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Guía superior
            Constraint::Length(8), // Modo de partición
            Constraint::Length(6), // Ruta de salida
            Constraint::Min(7),    // Modo de acción (Copiar vs Mover)
        ])
        .split(area);

    // 1. Guía superior
    let guide_lines = vec![
        Line::from(vec![
            Span::styled("CONFIGURACIÓN DE PARTICIÓN Y SALIDA", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" — Define cómo se nombrarán los archivos resultantes y el destino en disco", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
        Line::from(vec![
            Span::styled("[1/2/3] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Modo Partición  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[M] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Copiar/Mover  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[O] ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Editar Ruta Salida  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[Enter] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
            Span::styled("Siguiente", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];
    f.render_widget(Paragraph::new(guide_lines), chunks[0]);

    // 2. Modo de Partición
    let is_year = state.split.partition_mode == SplitPartitionMode::ByYear;
    let is_year_month = state.split.partition_mode == SplitPartitionMode::ByYearMonth;
    let is_single = state.split.partition_mode == SplitPartitionMode::SinglePst;

    let part_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(if is_year { "  (●) " } else { "  (○) " }, if is_year { Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
            Span::styled("[1] Un PST por cada año ", if is_year { Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MAIN) }),
            Span::styled("(ej: Archivo_2022.pst, Archivo_2023.pst) — Recomendado para archivos masivos", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(if is_year_month { "  (●) " } else { "  (○) " }, if is_year_month { Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
            Span::styled("[2] Un PST por cada año y mes ", if is_year_month { Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MAIN) }),
            Span::styled("(ej: Archivo_2024_01.pst, Archivo_2024_02.pst) — Ideal para segregación mensual", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(if is_single { "  (●) " } else { "  (○) " }, if is_single { Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
            Span::styled("[3] Un único archivo consolidado ", if is_single { Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MAIN) }),
            Span::styled("(ej: Archivo_filtrado.pst) — Extrae todo el rango filtrado en un solo PST", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];

    let part_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" 1. Modo de Partición ");
    f.render_widget(Paragraph::new(part_lines).block(part_block), chunks[1]);

    // 3. Carpeta de Salida
    let output_dir_style = if state.split.is_editing_output_dir {
        Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Theme::TEXT_MAIN)
    };

    let dir_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Carpeta de Guardado: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!("\"{}\"", state.split.output_dir), output_dir_style),
            Span::styled(if state.split.is_editing_output_dir { " ✎ [Escribe y pulsa Enter]" } else { "  (Por defecto: misma carpeta del archivo)" }, Style::default().fg(Theme::WARNING)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  💡 Presiona ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[O]", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" para modificar la ruta de destino de los nuevos archivos PST.", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];

    let dir_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if state.split.is_editing_output_dir { Theme::BRAND_PRIMARY } else { Theme::BORDER_INACTIVE }))
        .title(" 2. Ubicación de Destino ");
    f.render_widget(Paragraph::new(dir_lines).block(dir_block), chunks[2]);

    // 4. Modo de Acción (Copiar vs Mover)
    let is_copy = state.split.transfer_mode == SplitTransferMode::Copy;
    let is_move = state.split.transfer_mode == SplitTransferMode::Move;

    let mut action_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(if is_copy { "  (●) " } else { "  (○) " }, if is_copy { Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
            Span::styled("COPIAR CORREOS [Recomendado] ", if is_copy { Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MAIN) }),
            Span::styled("— Los correos se duplican hacia los nuevos PSTs y el archivo original permanece 100% intacto.", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(if is_move { "  (●) " } else { "  (○) " }, if is_move { Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
            Span::styled("MOVER CORREOS [Destructivo / Reducción] ", if is_move { Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MAIN) }),
            Span::styled("— Extrae los correos y los elimina del PST original para reducir su peso en disco.", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];

    if is_move {
        action_lines.push(Line::from(""));
        action_lines.push(Line::from(vec![
            Span::styled("  ⚠️ ADVERTENCIA: ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
            Span::styled("El archivo PST de origen se modificará. Asegúrate de tener una copia de respaldo antes de continuar.", Style::default().fg(Theme::WARNING)),
        ]));
    }

    let action_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if is_move { Theme::WARNING } else { Theme::BORDER_INACTIVE }))
        .title(" 3. Acción sobre el PST Original [Presiona M para alternar] ");
    f.render_widget(Paragraph::new(action_lines).block(action_block), chunks[3]);
}
