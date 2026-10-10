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
            Span::styled("PARTITION AND OUTPUT CONFIGURATION", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" — Define how output files are named and the destination folder on disk", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
        Line::from(vec![
            Span::styled("[1/2/3] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Partition Mode  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[M] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Copy/Move  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[O] ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("Edit Output Path  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[Enter] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
            Span::styled("Next", Style::default().fg(Theme::TEXT_MUTED)),
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
            Span::styled("[1] One PST per year ", if is_year { Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MAIN) }),
            Span::styled("(e.g., File_2022.pst, File_2023.pst) — Recommended for massive archives", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(if is_year_month { "  (●) " } else { "  (○) " }, if is_year_month { Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
            Span::styled("[2] One PST per year and month ", if is_year_month { Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MAIN) }),
            Span::styled("(e.g., File_2024_01.pst, File_2024_02.pst) — Ideal for monthly segregation", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(if is_single { "  (●) " } else { "  (○) " }, if is_single { Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
            Span::styled("[3] Single consolidated archive ", if is_single { Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MAIN) }),
            Span::styled("(e.g., File_filtered.pst) — Extracts entire filtered range into one PST", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];

    let part_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" 1. Partition Mode ");
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
            Span::styled("  Output Folder: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!("\"{}\"", state.split.output_dir), output_dir_style),
            Span::styled(if state.split.is_editing_output_dir { " ✎ [Type path and press Enter]" } else { "  (Default: same folder as original)" }, Style::default().fg(Theme::WARNING)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ※ Press ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[O]", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" to modify the destination path for the new PST files.", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];

    let dir_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if state.split.is_editing_output_dir { Theme::BRAND_PRIMARY } else { Theme::BORDER_INACTIVE }))
        .title(" 2. Destination Location ");
    f.render_widget(Paragraph::new(dir_lines).block(dir_block), chunks[2]);

    // 4. Modo de Acción (Copiar vs Mover)
    let is_copy = state.split.transfer_mode == SplitTransferMode::Copy;
    let is_move = state.split.transfer_mode == SplitTransferMode::Move;

    let mut action_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(if is_copy { "  (●) " } else { "  (○) " }, if is_copy { Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
            Span::styled("COPY EMAILS [Recommended] ", if is_copy { Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MAIN) }),
            Span::styled("— Emails are duplicated into new PSTs; original file remains 100% intact.", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(if is_move { "  (●) " } else { "  (○) " }, if is_move { Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MUTED) }),
            Span::styled("MOVE EMAILS [Destructive / Shrink] ", if is_move { Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_MAIN) }),
            Span::styled("— Extracts emails and deletes them from original PST to reduce disk size.", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];

    if is_move {
        action_lines.push(Line::from(""));
        action_lines.push(Line::from(vec![
            Span::styled("  ▲ WARNING: ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
            Span::styled("The source PST file will be modified. Ensure you have a backup copy before continuing.", Style::default().fg(Theme::WARNING)),
        ]));
    }

    let action_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if is_move { Theme::WARNING } else { Theme::BORDER_INACTIVE }))
        .title(" 3. Action on Source PST [Press M to toggle] ");
    f.render_widget(Paragraph::new(action_lines).block(action_block), chunks[3]);
}
