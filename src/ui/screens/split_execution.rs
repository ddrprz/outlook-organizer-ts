use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
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
            Constraint::Length(4), // Barra de progreso principal
            Constraint::Length(5), // Tarjeta de métricas en vivo
            Constraint::Min(8),    // Registro de actividad (Live log)
        ])
        .split(area);

    // 1. Barra de progreso principal
    let current_items = state.progress.current_pst_items;
    let total_items = state.progress.current_pst_total;

    let percent = if total_items > 0 {
        ((current_items as f64 / total_items as f64) * 100.0).clamp(0.0, 100.0) as u16
    } else {
        0
    };

    let title_str = if state.progress.graceful_cancelling {
        " [DETENIENDO CON SEGURIDAD... DESMONTANDO PSTs] "
    } else {
        " Generando y poblando archivos PST "
    };

    let gauge_label = format!(
        "{}% ({}/{} correos analizados)",
        percent, current_items, total_items
    );

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(if state.progress.graceful_cancelling {
                    Theme::WARNING
                } else {
                    Theme::ACCENT_PRIMARY
                }))
                .title(title_str),
        )
        .gauge_style(
            Style::default()
                .fg(if state.progress.graceful_cancelling {
                    Theme::WARNING
                } else {
                    Theme::BRAND_PRIMARY
                })
                .bg(Theme::SURFACE),
        )
        .percent(percent)
        .label(gauge_label);

    f.render_widget(gauge, chunks[0]);

    // 2. Tarjeta de Métricas en Vivo
    let metric_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(chunks[1]);

    let card_extracted = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(format!("{}", state.progress.imported_count), Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD))),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::SUCCESS))
            .title(" Correos Transferidos "),
    );
    f.render_widget(card_extracted, metric_chunks[0]);

    let card_speed = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(format!("{:.1} msg/s", state.progress.speed_mps), Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD))),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
            .title(" Velocidad "),
    );
    f.render_widget(card_speed, metric_chunks[1]);

    let eta_str = if state.progress.eta_seconds > 0 {
        format!("~{}", crate::ui::format::format_duration_compact(state.progress.eta_seconds))
    } else {
        "--".to_string()
    };
    let card_eta = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(eta_str, Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD))),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::BRAND_PRIMARY))
            .title(" Tiempo Estimado (ETA) "),
    );
    f.render_widget(card_eta, metric_chunks[2]);

    let current_dest_name = if state.progress.current_pst_name.is_empty() {
        "Iniciando...".to_string()
    } else {
        state.progress.current_pst_name.clone()
    };
    let card_file = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(current_dest_name, Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD))),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::BORDER_INACTIVE))
            .title(" PST Activo de Salida "),
    );
    f.render_widget(card_file, metric_chunks[3]);

    // 3. Registro de Actividad en Vivo (Live Log)
    let log_lines: Vec<Line> = state
        .activity_log
        .iter()
        .rev()
        .take(15)
        .rev()
        .map(|msg| {
            let style = if msg.contains("[ERROR]") {
                Style::default().fg(Theme::DANGER)
            } else if msg.contains("[WARN]") {
                Style::default().fg(Theme::WARNING)
            } else if msg.contains("Finalizado") || msg.contains("exitosamente") {
                Style::default().fg(Theme::SUCCESS)
            } else {
                Style::default().fg(Theme::TEXT_MUTED)
            };
            Line::from(Span::styled(msg, style))
        })
        .collect();

    let log_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER_INACTIVE))
        .title(" Registro de Actividad en Tiempo Real ");
    f.render_widget(Paragraph::new(log_lines).block(log_block), chunks[2]);
}
