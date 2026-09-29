use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
    Frame,
};

use crate::{
    app::{AppState, RoutingGranularity, TransferMode},
    ui::theme::Theme,
};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let progress = &state.progress;
    let selected_psts = state.selected_psts();
    let total_psts = if progress.total_psts > 0 {
        progress.total_psts
    } else if !selected_psts.is_empty() {
        selected_psts.len()
    } else {
        1
    };
    let is_single_pst = total_psts <= 1;

    // Distribución vertical adaptable según si es 1 PST (1 barra) o varios PSTs (2 barras)
    let chunks = if is_single_pst {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // 1. Ribbon de Estado y Contexto MAPI
                Constraint::Length(5), // 2. Barra ÚNICA de Progreso para 1 PST
                Constraint::Min(8),    // 3. Panel de Métricas KPI y Log en Vivo
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // 1. Ribbon de Estado y Contexto MAPI
                Constraint::Length(4), // 2. Barra 1: PST Actual
                Constraint::Length(4), // 3. Barra 2: Progreso Global Consolidado
                Constraint::Min(8),    // 4. Panel de Métricas KPI y Log en Vivo
            ])
            .split(area)
    };

    // =========================================================================
    // 1. RIBBON DE ESTADO Y CONTEXTO MAPI
    // =========================================================================
    let status_border_color = if progress.graceful_cancelling || progress.throttling_active {
        Theme::WARNING
    } else {
        Theme::ACCENT_SECONDARY
    };

    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(status_border_color))
        .title(" ◈ Estado de la Sesión MAPI y Destino ◈ ");

    let status_inner = status_block.inner(chunks[0]);
    f.render_widget(status_block, chunks[0]);

    let status_badge = if progress.graceful_cancelling {
        Span::styled(
            " ■ CANCELANDO CON SEGURIDAD ",
            Style::default()
                .bg(Theme::WARNING)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
    } else if progress.throttling_active {
        Span::styled(
            " ▲ THROTTLING ACTIVO ",
            Style::default()
                .bg(Theme::WARNING)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " ● EN EJECUCIÓN ACTIVA ",
            Style::default()
                .bg(Theme::SUCCESS)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
    };

    let selected_mboxes = state.selected_mailboxes();
    let mbox_names = if selected_mboxes.is_empty() {
        "Buzón predeterminado".to_string()
    } else if selected_mboxes.len() == 1 {
        selected_mboxes[0].display_name.clone()
    } else {
        selected_mboxes.iter().map(|m| m.display_name.as_str()).collect::<Vec<_>>().join(", ")
    };

    let mode_badge = match state.transfer_mode {
        TransferMode::Copy => Span::styled("Modo: Copiar (Intacto)", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        TransferMode::Move => Span::styled("Modo: Mover (Transaccional)", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
    };
    let routing_badge = match state.routing_granularity {
        RoutingGranularity::Mirror => Span::styled("Estructura: Nativa Espejo", Style::default().fg(Theme::TEXT_MUTED)),
        RoutingGranularity::Years => Span::styled("Estructura: Por Años", Style::default().fg(Theme::ACCENT_SECONDARY)),
        RoutingGranularity::YearsAndMonths => Span::styled("Estructura: Años y Meses", Style::default().fg(Theme::BRAND_PRIMARY)),
    };

    let ribbon_line = Line::from(vec![
        status_badge,
        Span::raw("  "),
        Span::styled("Destino: ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(mbox_names, Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
        Span::styled("  •  ", Style::default().fg(Theme::TEXT_MUTED)),
        mode_badge,
        Span::styled("  •  ", Style::default().fg(Theme::TEXT_MUTED)),
        routing_badge,
    ]);
    f.render_widget(Paragraph::new(ribbon_line), status_inner);

    // =========================================================================
    // 2. BARRAS DE PROGRESO (1 BARRA si es 1 PST, 2 BARRAS si son varios)
    // =========================================================================
    let current_file_name = if !progress.current_pst_name.is_empty() {
        progress.current_pst_name.as_str()
    } else if let Some(first) = selected_psts.first() {
        first.name.as_str()
    } else {
        "Archivo PST"
    };

    let pst_percent = if progress.current_pst_total > 0 {
        ((progress.current_pst_items as f64 / progress.current_pst_total as f64) * 100.0).clamp(0.0, 100.0) as u16
    } else {
        0
    };

    let bottom_area = if is_single_pst {
        // --- CASO 1: UN SOLO PST -> UNA SOLA BARRA MODERNA Y COMPLETA ---
        let label = format!(
            " {}%  •  {}/{} correos  •  Velocidad: {:.1} msgs/s  •  ETA: ~{}s ",
            pst_percent,
            progress.current_pst_items,
            progress.current_pst_total,
            progress.speed_mps,
            progress.eta_seconds
        );

        let single_gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(Theme::BRAND_PRIMARY))
                    .title(format!(" ◈ Progreso de Importación del Archivo: {} ◈ ", current_file_name)),
            )
            .gauge_style(
                Style::default()
                    .fg(Theme::BRAND_PRIMARY)
                    .bg(Theme::BG_CARD)
                    .add_modifier(Modifier::BOLD),
            )
            .percent(pst_percent)
            .label(label);

        f.render_widget(single_gauge, chunks[1]);
        chunks[2]
    } else {
        // --- CASO 2: MÚLTIPLES PSTs -> DOS BARRAS (PST ACTUAL + GLOBAL) ---
        let current_idx = progress.current_pst_idx.max(1);

        // Barra 1: PST Actual
        let pst_label = format!(
            " {}%  ({}/{} correos) ",
            pst_percent, progress.current_pst_items, progress.current_pst_total
        );
        let pst_gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
                    .title(format!(" ◈ Archivo Actual ({}/{}): {} ◈ ", current_idx, total_psts, current_file_name)),
            )
            .gauge_style(
                Style::default()
                    .fg(Theme::ACCENT_PRIMARY)
                    .bg(Theme::BG_CARD)
                    .add_modifier(Modifier::BOLD),
            )
            .percent(pst_percent)
            .label(pst_label);
        f.render_widget(pst_gauge, chunks[1]);

        // Barra 2: Progreso Global Consolidado
        let global_percent = if progress.global_items_total > 0 {
            ((progress.global_items_processed as f64 / progress.global_items_total as f64) * 100.0).clamp(0.0, 100.0) as u16
        } else {
            0
        };
        let global_label = format!(
            " {}%  •  Total Global: {}/{} correos  •  Vel: {:.1} msgs/s  •  ETA: ~{}s ",
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
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(Theme::SUCCESS))
                    .title(" ◈ Progreso Global Consolidado (Todos los PSTs) ◈ "),
            )
            .gauge_style(
                Style::default()
                    .fg(Theme::SUCCESS)
                    .bg(Theme::BG_CARD)
                    .add_modifier(Modifier::BOLD),
            )
            .percent(global_percent)
            .label(global_label);
        f.render_widget(global_gauge, chunks[2]);

        chunks[3]
    };

    // =========================================================================
    // 3. PANEL DE MÉTRICAS KPI (42%) Y LOG EN VIVO CON AUTO-SCROLL (58%)
    // =========================================================================
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(42), // Métricas KPI y Estado del Sistema
            Constraint::Percentage(58), // Log en Directo con Auto-Scroll
        ])
        .split(bottom_area);

    // --- Panel Izquierdo: Métricas en Directo con Símbolos Uniformes ---
    let metrics_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
        .title(" ◈ Métricas en Directo ◈ ");

    let metrics_inner = metrics_block.inner(bottom_chunks[0]);
    f.render_widget(metrics_block, bottom_chunks[0]);

    let metrics_lines = vec![
        Line::from(vec![
            Span::styled("✓ Transferidos:      ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{} correos", progress.imported_count),
                Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("⧉ Duplicados Omit.:  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{} correos", progress.duplicates_skipped),
                Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("▲ Errores Lectura:   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", progress.error_count),
                if progress.error_count > 0 {
                    Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_MUTED)
                },
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("▸ Velocidad MAPI:    ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{:.1} correos/seg", progress.speed_mps),
                Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("◷ Tiempo Estimado:   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("~{} segundos", progress.eta_seconds),
                Style::default().fg(Theme::ACCENT_PRIMARY),
            ),
        ]),
        Line::from(vec![
            Span::styled("◆ Anti-Throttling:   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                if progress.throttling_active {
                    "▲ Pausa activa (429 Backoff)"
                } else {
                    "● Normal (Velocidad óptima)"
                },
                Style::default()
                    .fg(if progress.throttling_active { Theme::WARNING } else { Theme::SUCCESS })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("◈ Integridad PST:    ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("Desmontaje seguro activo", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];
    f.render_widget(Paragraph::new(metrics_lines), metrics_inner);

    // --- Panel Derecho: Log en Directo con Auto-Scroll y Sintaxis ---
    let log_title = if progress.graceful_cancelling {
        " ▲ REGISTRO: PARADA SEGURA EN CURSO (DESMONTANDO PST) "
    } else {
        " ◈ Registro de Actividad y Telemetría MAPI ◈ "
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

    // Auto-scroll: calcular las líneas visibles para mostrar siempre los eventos más recientes
    let available_lines = log_inner.height as usize;
    let total_log_entries = state.activity_log.len();
    let skip_count = total_log_entries.saturating_sub(available_lines);

    let log_lines: Vec<Line> = if state.activity_log.is_empty() {
        vec![
            Line::from(Span::styled("  [SISTEMA] Iniciando sesión MAPI y preparando almacenes de Outlook...", Style::default().fg(Theme::TEXT_MUTED))),
        ]
    } else {
        state.activity_log
            .iter()
            .skip(skip_count)
            .map(|s| {
                let style = if s.starts_with("[FIN]") || s.starts_with("[AUDITORÍA]") {
                    Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)
                } else if s.starts_with("[ERROR]") {
                    Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD)
                } else if s.starts_with("[WARN]") || s.starts_with("[THROTTLING]") {
                    Style::default().fg(Theme::WARNING)
                } else if s.starts_with("[SISTEMA]") {
                    Style::default().fg(Theme::BRAND_PRIMARY)
                } else {
                    Style::default().fg(Theme::TEXT_MAIN)
                };
                Line::from(Span::styled(format!("  {}", s), style))
            })
            .collect()
    };

    f.render_widget(Paragraph::new(log_lines), log_inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use crate::app::{AppState, PstItem};

    #[test]
    fn test_render_execution_single_pst() {
        let mut state = AppState::new();
        state.discovered_psts = vec![PstItem {
            name: "test.pst".to_string(),
            path: r"C:\Correo\test.pst".to_string(),
            size_mb: 50.0,
            selected: true,
        }];
        state.progress.current_pst_name = "test.pst".to_string();
        state.progress.current_pst_items = 45;
        state.progress.current_pst_total = 100;
        state.progress.speed_mps = 12.5;
        state.progress.eta_seconds = 4;
        state.log_event("[SISTEMA] Iniciando importación...".to_string());

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| {
            render(f, f.area(), &state);
        }).unwrap();
    }

    #[test]
    fn test_render_execution_multi_pst() {
        let mut state = AppState::new();
        state.discovered_psts = vec![
            PstItem {
                name: "test1.pst".to_string(),
                path: r"C:\Correo\test1.pst".to_string(),
                size_mb: 50.0,
                selected: true,
            },
            PstItem {
                name: "test2.pst".to_string(),
                path: r"C:\Correo\test2.pst".to_string(),
                size_mb: 80.0,
                selected: true,
            },
        ];
        state.progress.total_psts = 2;
        state.progress.current_pst_idx = 1;
        state.progress.current_pst_name = "test1.pst".to_string();
        state.progress.current_pst_items = 50;
        state.progress.current_pst_total = 100;
        state.progress.global_items_processed = 50;
        state.progress.global_items_total = 200;
        state.progress.speed_mps = 15.0;
        state.progress.eta_seconds = 10;
        state.progress.throttling_active = true;
        state.log_event("[WARN] Throttling activo en Exchange".to_string());

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| {
            render(f, f.area(), &state);
        }).unwrap();
    }

    #[test]
    fn test_render_execution_cancelling_state() {
        let mut state = AppState::new();
        state.progress.graceful_cancelling = true;
        state.log_event("[SISTEMA] Solicitud de parada segura recibida".to_string());

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| {
            render(f, f.area(), &state);
        }).unwrap();
    }
}
