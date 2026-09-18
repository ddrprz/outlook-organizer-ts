use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
    Frame,
};

use crate::{app::AppState, ui::theme::Theme};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Barra 1: PST Actual
            Constraint::Length(4), // Barra 2: Progreso Global
            Constraint::Min(6),    // Panel métricas y logs
        ])
        .split(area);

    let progress = &state.progress;

    // 1. Barra 1: PST Actual
    let pst_percent = if progress.current_pst_total > 0 {
        ((progress.current_pst_items as f64 / progress.current_pst_total as f64) * 100.0) as u16
    } else {
        0
    };
    let pst_label = format!(
        "{}% ({}/{} correos) - Archivo: {}",
        pst_percent,
        progress.current_pst_items,
        progress.current_pst_total,
        if progress.current_pst_name.is_empty() { "Preparando importación..." } else { &progress.current_pst_name }
    );
    let pst_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
                .title(" PST Actual "),
        )
        .gauge_style(Style::default().fg(Theme::ACCENT_PRIMARY).bg(Theme::BG_CARD))
        .percent(pst_percent)
        .label(pst_label);
    f.render_widget(pst_gauge, chunks[0]);

    // 2. Barra 2: Progreso Global
    let global_percent = if progress.global_items_total > 0 {
        ((progress.global_items_processed as f64 / progress.global_items_total as f64) * 100.0) as u16
    } else {
        0
    };
    let global_label = format!(
        "{}% (Total: {}/{} correos) - Vel: {:.1} msgs/s - ETA: ~{}s",
        global_percent,
        progress.global_items_processed,
        progress.global_items_total,
        progress.speed_mps,
        progress.eta_seconds
    );
    let global_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Theme::SUCCESS))
                .title(" Progreso Global "),
        )
        .gauge_style(Style::default().fg(Theme::SUCCESS).bg(Theme::BG_CARD))
        .percent(global_percent)
        .label(global_label);
    f.render_widget(global_gauge, chunks[1]);

    // 3. Métricas y Logs en Vivo
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Métricas
            Constraint::Percentage(60), // Log en directo
        ])
        .split(chunks[2]);

    // Métricas
    let metrics_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
        .title(" Métricas en Directo ");

    let metrics_inner = metrics_block.inner(bottom_chunks[0]);
    f.render_widget(metrics_block, bottom_chunks[0]);

    let metrics_lines = vec![
        Line::from(vec![
            Span::styled("• Importados con éxito: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!("{}", progress.imported_count), Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("• Duplicados omitidos:   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!("{}", progress.duplicates_skipped), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("• Errores de lectura:    ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(format!("{}", progress.error_count), Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("• Estado de Throttling:  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(if progress.throttling_active { "⚠️ Limitando (Backoff)" } else { "● Normal (Óptimo)" }, Style::default().fg(if progress.throttling_active { Theme::WARNING } else { Theme::SUCCESS })),
        ]),
    ];
    f.render_widget(Paragraph::new(metrics_lines), metrics_inner);

    // Logs en directo
    let log_title = if progress.graceful_cancelling {
        " ⚠️ REGISTRO: CANCELANDO CON SEGURIDAD (DESMONTANDO PST) "
    } else {
        " Registro de Actividad en Vivo "
    };
    let log_border_color = if progress.graceful_cancelling {
        Theme::WARNING
    } else {
        Theme::ACCENT_PRIMARY
    };

    let log_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(log_border_color))
        .title(log_title);

    let log_inner = log_block.inner(bottom_chunks[1]);
    f.render_widget(log_block, bottom_chunks[1]);

    let log_lines: Vec<Line> = if state.activity_log.is_empty() {
        vec![
            Line::from(Span::styled("[SISTEMA] Conectando con sesión MAPI de Outlook...", Style::default().fg(Theme::TEXT_MUTED))),
        ]
    } else {
        state.activity_log.iter().map(|s| Line::from(s.as_str())).collect()
    };

    f.render_widget(Paragraph::new(log_lines), log_inner);
}
