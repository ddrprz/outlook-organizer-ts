use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::{
    app::AppState,
    ui::{screens::mailbox::build_storage_bar_cell, theme::Theme},
};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let has_warning = state.transfer.warning_notice.is_some();
    let show_cards = area.height >= 24;

    let chunks = if has_warning {
        if show_cards {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4), // Header / Badges resumen
                    Constraint::Length(3), // Alerta de advertencia
                    Constraint::Min(8),    // Tabla de buzones
                    Constraint::Length(3), // Footer de atajos
                ])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Alerta
                    Constraint::Min(6),    // Tabla
                    Constraint::Length(2), // Footer
                ])
                .split(area)
        }
    } else if show_cards {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // Header resumen Origen & Destino
                Constraint::Min(8),    // Tabla de buzones
                Constraint::Length(3), // Footer de atajos
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(6),    // Tabla
                Constraint::Length(2), // Footer
            ])
            .split(area)
    };

    let (cards_area, table_area, footer_area) = if has_warning {
        if show_cards {
            (Some(chunks[0]), chunks[2], chunks[3])
        } else {
            (None, chunks[1], chunks[2])
        }
    } else if show_cards {
        (Some(chunks[0]), chunks[1], chunks[2])
    } else {
        (None, chunks[0], chunks[1])
    };

    // Renderizar advertencia si existe
    if let Some(ref notice) = state.transfer.warning_notice {
        let warn_idx = if show_cards { 1 } else { 0 };
        let warn_p = Paragraph::new(Line::from(vec![
            Span::styled(" ▲ ", Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD)),
            Span::styled(notice, Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Theme::DANGER)),
        );
        f.render_widget(warn_p, chunks[warn_idx]);
    }

    // =========================================================================
    // 1. TARJETAS DE RESUMEN: ORIGEN Y DESTINO SELECCIONADOS
    // =========================================================================
    if let Some(c_area) = cards_area {
        let split_cards = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(c_area);

        // Tarjeta Origen
        let src_mbox = state.get_transfer_source_mailbox();
        let src_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if src_mbox.is_some() {
                Style::default().fg(Theme::BRAND_PRIMARY)
            } else {
                Style::default().fg(Theme::BORDER_INACTIVE)
            })
            .title(" [S] Source Mailbox (Extract) ");

        let src_inner = src_block.inner(split_cards[0]);
        f.render_widget(src_block, split_cards[0]);

        let src_lines = if let Some(src) = src_mbox {
            vec![
                Line::from(vec![
                    Span::styled("◈ ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                    Span::styled(&src.display_name, Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled("  Type: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(&src.store_type, Style::default().fg(Theme::ACCENT_PRIMARY)),
                    Span::styled(" • Used: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(format!("{:.1} GB", src.get_used_gb()), Style::default().fg(Theme::TEXT_MAIN)),
                    if state.transfer.is_inspecting_source {
                        Span::styled("  [Analyzing...]", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD))
                    } else {
                        Span::raw("")
                    },
                ]),
            ]
        } else {
            vec![
                Line::from(Span::styled("No source selected", Style::default().fg(Theme::WARNING))),
                Line::from(Span::styled("Press [S] on focused mailbox to set as source", Style::default().fg(Theme::TEXT_MUTED))),
            ]
        };
        f.render_widget(Paragraph::new(src_lines), src_inner);

        // Tarjeta Destino
        let dst_mbox = state.get_transfer_target_mailbox();
        let dst_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if dst_mbox.is_some() {
                Style::default().fg(Theme::SUCCESS)
            } else {
                Style::default().fg(Theme::BORDER_INACTIVE)
            })
            .title(" [T] Target Mailbox (Deposit) ");

        let dst_inner = dst_block.inner(split_cards[1]);
        f.render_widget(dst_block, split_cards[1]);

        let dst_lines = if let Some(dst) = dst_mbox {
            vec![
                Line::from(vec![
                    Span::styled("◈ ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
                    Span::styled(&dst.display_name, Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled("  Type: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(&dst.store_type, Style::default().fg(Theme::ACCENT_PRIMARY)),
                    Span::styled(" • Free: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(format!("{:.1} GB / {}", dst.get_free_gb(), dst.get_quota_display()), Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
                ]),
            ]
        } else {
            vec![
                Line::from(Span::styled("No target selected", Style::default().fg(Theme::WARNING))),
                Line::from(Span::styled("Press [T] on focused mailbox to set as target", Style::default().fg(Theme::TEXT_MUTED))),
            ]
        };
        f.render_widget(Paragraph::new(dst_lines), dst_inner);
    }

    // =========================================================================
    // 2. TABLA DE BUZONES DISPONIBLES
    // =========================================================================
    if state.is_loading_mailboxes && state.discovered_mailboxes.is_empty() {
        let loading_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
            .title(" Outlook MAPI Stores ");
        let loading_p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("⧗ Connecting to Outlook and detecting mailboxes...", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("Querying MAPI accounts, shared mailboxes, and storage limits.", Style::default().fg(Theme::TEXT_MUTED))),
        ])
        .alignment(Alignment::Center)
        .block(loading_block);
        f.render_widget(loading_p, table_area);
    } else {
        let header_cells = [
            "Role",
            "Mailbox / Store",
            "Store Type",
            "Used / Quota",
            "Capacity (Bar)",
            "Health",
        ]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)));
        let header = Row::new(header_cells).height(1).bottom_margin(1);

        let rows = state.discovered_mailboxes.iter().enumerate().map(|(idx, item)| {
            let is_focused = idx == state.transfer.table_cursor;
            let is_src = state.transfer.source_mailbox_idx == Some(idx);
            let is_dst = state.transfer.target_mailbox_idx == Some(idx);

            let (role_text, role_style) = if is_src {
                ("[SRC]", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD))
            } else if is_dst {
                ("[DST]", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD))
            } else {
                (" ( ) ", Style::default().fg(Theme::TEXT_MUTED))
            };
            let role_cell = Cell::from(role_text).style(role_style);

            let name_prefix = if is_focused { "▶ " } else { "  " };
            let name_cell = Cell::from(format!("{}{}", name_prefix, item.display_name));

            let type_cell = Cell::from(format!("({})", item.store_type)).style(
                if item.store_type == "ExchangeOnline" {
                    Style::default().fg(Theme::BRAND_PRIMARY)
                } else if item.store_type == "SharedMailbox" {
                    Style::default().fg(Theme::ACCENT_PRIMARY)
                } else {
                    Style::default().fg(Theme::TEXT_MUTED)
                }
            );

            let usage_ratio_str = format!("{:.1} GB / {}", item.get_used_gb(), item.get_quota_display());
            let usage_cell = Cell::from(usage_ratio_str).style(Style::default().fg(Theme::TEXT_MAIN));

            let pct = item.get_usage_percent();
            let bar_cell = build_storage_bar_cell(pct, is_focused);

            let (health_label, health_color) = item.health_status();
            let health_symbol = if pct >= 90.0 {
                "✖ "
            } else if pct >= 75.0 {
                "▲ "
            } else {
                "● "
            };
            let health_cell = Cell::from(format!("{}{}", health_symbol, health_label))
                .style(Style::default().fg(health_color).add_modifier(if pct >= 75.0 { Modifier::BOLD } else { Modifier::empty() }));

            let row = Row::new(vec![role_cell, name_cell, type_cell, usage_cell, bar_cell, health_cell]);
            if is_focused {
                row.style(Style::default().bg(Theme::BG_CARD).fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD))
            } else {
                row
            }
        });

        let table = Table::new(
            rows,
            [
                Constraint::Length(7),  // Role [SRC] / [DST]
                Constraint::Min(24),    // Buzón / Almacén
                Constraint::Length(16), // Tipo
                Constraint::Length(18), // Uso / Cuota
                Constraint::Length(23), // Barra Ocupación
                Constraint::Length(12), // Salud
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
                .title(" Available Outlook / Exchange Mailboxes "),
        );

        f.render_widget(table, table_area);
    }

    // =========================================================================
    // 3. FOOTER DE ATAJOS Y CONTEXTO
    // =========================================================================
    let help_line = Line::from(vec![
        Span::styled("[↑/↓] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Navigate   ", Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled("[S] ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Set Source   ", Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled("[T] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        Span::styled("Set Target   ", Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled("[D] ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
        Span::styled("Mailbox Details   ", Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled("[R] ", Style::default().fg(Theme::ACCENT_SECONDARY).add_modifier(Modifier::BOLD)),
        Span::styled("Refresh   ", Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled("[Enter] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        Span::styled("Continue   ", Style::default().fg(Theme::TEXT_MAIN)),
        Span::styled("[Esc] ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("Main Menu", Style::default().fg(Theme::TEXT_MUTED)),
    ]);

    let footer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER_INACTIVE));
    let footer_p = Paragraph::new(help_line)
        .alignment(Alignment::Center)
        .block(footer_block);
    f.render_widget(footer_p, footer_area);
}
