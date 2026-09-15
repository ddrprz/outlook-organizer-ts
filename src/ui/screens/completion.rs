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
            Constraint::Length(7), // Estado final
            Constraint::Min(8),    // Opciones de reporte
            Constraint::Length(3), // Cierre
        ])
        .split(area);

    // 1. Estado Final
    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::SUCCESS))
        .title(" Resumen de Finalización ");

    let status_inner = status_block.inner(chunks[0]);
    f.render_widget(status_block, chunks[0]);

    let status_lines = vec![
        Line::from(vec![
            Span::styled("Estado de la Operación: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("COMPLETADO EXITOSAMENTE", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("Total Correos: {} | ", state.progress.imported_count + state.progress.duplicates_skipped), Style::default().fg(Theme::TEXT_MAIN)),
            Span::styled(format!("Importados: {} | ", state.progress.imported_count), Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
            Span::styled(format!("Duplicados Omitidos: {} | ", state.progress.duplicates_skipped), Style::default().fg(Theme::BRAND_PRIMARY)),
            Span::styled(format!("Errores: {}", state.progress.error_count), Style::default().fg(if state.progress.error_count > 0 { Theme::DANGER } else { Theme::SUCCESS })),
        ]),
    ];
    f.render_widget(Paragraph::new(status_lines), status_inner);

    // 2. Reporte JSON y HTML
    let report_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Auditoría y Generación de Reportes ");

    let report_inner = report_block.inner(chunks[1]);
    f.render_widget(report_block, chunks[1]);

    let report_lines = vec![
        Line::from(vec![
            Span::styled("• Registro JSON de Auditoría: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("Generado automáticamente en carpeta fechada (Modo Producción .exe)", Style::default().fg(Theme::TEXT_MAIN)),
        ]),
        Line::from(Span::styled("  Ubicación: .\\logs\\2026-09-15\\run_audit.json", Style::default().fg(Theme::BRAND_PRIMARY))),
        Line::from(""),
        Line::from(vec![
            Span::styled("• Informe Visual HTML: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("¿Desea generar el informe interactivo HTML con diseño web moderno?", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  [H] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
            Span::styled("Generar Informe HTML en carpeta predeterminada", Style::default().fg(Theme::TEXT_MAIN)),
        ]),
        Line::from(vec![
            Span::styled("  [C] ", Style::default().fg(Theme::ACCENT_SECONDARY).add_modifier(Modifier::BOLD)),
            Span::styled("Especificar ruta personalizada de guardado", Style::default().fg(Theme::TEXT_MAIN)),
        ]),
    ];
    f.render_widget(Paragraph::new(report_lines), report_inner);

    // 3. Salida
    let exit_line = Line::from(Span::styled("Presiona [Enter] o [Q] para cerrar la aplicación.", Style::default().fg(Theme::TEXT_MUTED)));
    f.render_widget(Paragraph::new(exit_line).alignment(Alignment::Center), chunks[2]);
}
