use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{app::AppState, ui::theme::Theme};

pub const MENU_ITEMS: [(&str, &str, &str); 4] = [
    (
        "Importar PST",
        "Iniciar el asistente completo de migración hacia buzones de Outlook / M365.",
        "Asistente guiado paso a paso con deduplicación y enrutamiento.",
    ),
    (
        "Escanear PSTs",
        "Explorar y listar los archivos .pst detectados en C:\\Correo o rutas locales.",
        "Detección de archivos, cálculo de tamaño y verificación de bloqueos.",
    ),
    (
        "Perfil MAPI",
        "Configurar el perfil de Outlook (predeterminado de Windows o personalizado).",
        "Alternar entre perfil del sistema o ingresar nombre de perfil MAPI.",
    ),
    (
        "Salir",
        "Cerrar la aplicación de forma limpia y segura.",
        "Restaura el modo de terminal y finaliza el proceso.",
    ),
];

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let is_tall = area.height >= 26;

    let (banner_area, badges_area, menu_area) = if is_tall {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // Espaciador superior
                Constraint::Length(11), // Banner ASCII Slant completo
                Constraint::Length(2),  // Espaciador generoso entre ASCII y Badges
                Constraint::Length(1),  // Badges y Tagline
                Constraint::Length(1),  // Espaciador entre Badges y Menú
                Constraint::Min(8),     // Menú interactivo + Tarjeta descriptiva
            ])
            .split(area);
        (chunks[1], chunks[3], chunks[5])
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Banner compacto
                Constraint::Length(1),  // Espacio
                Constraint::Length(1),  // Badges y Tagline
                Constraint::Length(1),  // Espacio
                Constraint::Min(7),     // Menú interactivo
            ])
            .split(area);
        (chunks[0], chunks[2], chunks[4])
    };

    // 1. BANNER ASCII
    if is_tall {
        let banner_lines = vec![
            Line::from(Span::styled("    ____        __  __            __  ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("   / __ \\__  __/ /_/ /___  ____  / /__", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("  / / / / / / / __/ / __ \\/ __ \\/ //_/", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(" / /_/ / /_/ / /_/ / /_/ / /_/ / ,<   ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(" \\____/\\__,_/\\__/_/\\____/\\____/_/|_|  ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("   ____                        _              ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("  / __ \\_________ _____ _____ (_)___  ___  _____", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(" / / / / ___/ __ `/ __ `/ __ `/ /_  / / _ \\/ ___/", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("/ /_/ / /  / /_/ / /_/ / / / / / / /_/  __/ /    ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("\\____/_/   \\__, /\\__,_/_/ /_/_/ /___/\\___/_/     ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("          /____/                                 ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD))),
        ];
        f.render_widget(Paragraph::new(banner_lines).alignment(Alignment::Center), banner_area);
    } else {
        let compact_banner = vec![
            Line::from(vec![
                Span::styled("◈ OUTLOOK ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled("ORGANIZER TS", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled(" — Herramienta de Migración Empresarial", Style::default().fg(Theme::TEXT_MUTED)),
            ]),
        ];
        f.render_widget(Paragraph::new(compact_banner).alignment(Alignment::Center), banner_area);
    }

    // 2. TAGLINE Y BADGES DE ESTADO (Con espaciado balanceado)
    let badges_line = Line::from(vec![
        Span::styled("[ ● MAPI Conectado ]", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        Span::styled("  ", Style::default()),
        Span::styled("[ ◈ Cero Riesgo de Corrupción ]", Style::default().fg(Theme::BRAND_PRIMARY)),
        Span::styled("  ", Style::default()),
        Span::styled("[ ⚡ Throttling M365 ]", Style::default().fg(Theme::WARNING)),
        Span::styled("  ", Style::default()),
        Span::styled(
            if state.use_default_profile { "[ Perfil: Predeterminado ]" } else { "[ Perfil: Manual ]" },
            Style::default().fg(Theme::TEXT_MUTED),
        ),
    ]);
    f.render_widget(Paragraph::new(badges_line).alignment(Alignment::Center), badges_area);

    // 3. MENÚ PRINCIPAL Y TARJETA CONTEXTUAL (Centrado horizontal con márgenes limpios)
    let body_area = if area.width > 100 {
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(4),
                Constraint::Percentage(92),
                Constraint::Percentage(4),
            ])
            .split(menu_area);
        h_chunks[1]
    } else {
        menu_area
    };

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Menú interactivo
            Constraint::Percentage(50), // Tarjeta descriptiva
        ])
        .split(body_area);

    // Renderizado del Menú
    let menu_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Menú Principal ");
    let menu_inner = menu_block.inner(body_chunks[0]);
    f.render_widget(menu_block, body_chunks[0]);

    let mut menu_lines = Vec::new();
    menu_lines.push(Line::from(""));

    for (idx, (title, _, _)) in MENU_ITEMS.iter().enumerate() {
        let is_selected = idx == state.welcome_menu_idx;
        let num_str = format!("[{}]", idx + 1);

        if is_selected {
            menu_lines.push(Line::from(vec![
                Span::styled(" ▶ ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:<4} ", num_str), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!(" {:<20} ", title),
                    Style::default()
                        .bg(Theme::ACCENT_PRIMARY)
                        .fg(Theme::BG_DARK)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        } else {
            menu_lines.push(Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::styled(format!("{:<4} ", num_str), Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(
                    format!(" {:<20} ", title),
                    Style::default().fg(Theme::TEXT_MAIN),
                ),
            ]));
        }
        menu_lines.push(Line::from(""));
    }

    f.render_widget(Paragraph::new(menu_lines), menu_inner);

    // Renderizado de la Tarjeta Contextual
    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
        .title(" Información de la Acción ");
    let detail_inner = detail_block.inner(body_chunks[1]);
    f.render_widget(detail_block, body_chunks[1]);

    let current_menu = MENU_ITEMS[state.welcome_menu_idx.min(MENU_ITEMS.len() - 1)];
    let detail_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Acción: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(current_menu.0, Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(Span::styled(current_menu.1, Style::default().fg(Theme::TEXT_MAIN))),
        Line::from(""),
        Line::from(Span::styled(current_menu.2, Style::default().fg(Theme::TEXT_MUTED))),
        Line::from(""),
        Line::from(vec![
            Span::styled("💡 Tip: ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
            Span::styled("Presiona ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[Enter]", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" para ejecutar o ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[1-4]", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(" para selección directa.", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];
    f.render_widget(Paragraph::new(detail_lines), detail_inner);
}
