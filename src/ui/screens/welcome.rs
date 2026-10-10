use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::{app::AppState, ui::theme::Theme};

pub const MENU_ITEMS: [(&str, &str, &str); 5] = [
    (
        "Import",
        "Launch the migration wizard to import PST emails into Outlook / M365 mailboxes.",
        "Step-by-step guided wizard with deduplication and temporal routing.",
    ),
    (
        "Scan",
        "Explore and inspect .pst files detected in C:\\Correo or local drives.",
        "File discovery, size calculation, and lock verification.",
    ),
    (
        "Split",
        "Split a PST file by filtering years and months into one or multiple PST archives.",
        "Generate new PST archives partitioned by year or month while preserving the original.",
    ),
    (
        "Profile",
        "Configure Outlook MAPI profile (Windows system default or custom).",
        "Select the active system profile or specify a custom profile name.",
    ),
    (
        "Quit",
        "Safely and cleanly close the application.",
        "Restores terminal state and terminates the process.",
    ),
];

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let is_tall = area.height >= 26;

    let (banner_area, badges_area, menu_area) = if is_tall {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // Top spacer
                Constraint::Length(11), // Full Slant ASCII Banner
                Constraint::Length(2),  // Spacer between ASCII and Badges
                Constraint::Length(1),  // Badges & Tagline
                Constraint::Length(1),  // Spacer between Badges and Menu
                Constraint::Min(8),     // Interactive menu + Detail card
            ])
            .split(area);
        (chunks[1], chunks[3], chunks[5])
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Compact banner
                Constraint::Length(1),  // Space
                Constraint::Length(1),  // Badges & Tagline
                Constraint::Length(1),  // Space
                Constraint::Min(7),     // Interactive menu
            ])
            .split(area);
        (chunks[0], chunks[2], chunks[4])
    };

    // 1. ASCII BANNER
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
                Span::styled(" — Enterprise Migration & PST Management Tool", Style::default().fg(Theme::TEXT_MUTED)),
            ]),
        ];
        f.render_widget(Paragraph::new(compact_banner).alignment(Alignment::Center), banner_area);
    }

    // 2. TAGLINE & STATUS BADGES
    let (profile_badge_text, profile_badge_style) = if state.use_default_profile {
        ("[ Profile: Default ]".to_string(), Style::default().fg(Theme::TEXT_MUTED))
    } else if state.custom_profile_name.trim().is_empty() {
        ("[ Profile: Manual (Unset) ]".to_string(), Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD))
    } else {
        (format!("[ Profile: {} ]", state.custom_profile_name), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD))
    };

    let badges_line = Line::from(vec![
        Span::styled("[ ● MAPI Connected ]", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        Span::styled("  ", Style::default()),
        Span::styled("[ ◈ Zero Corruption Risk ]", Style::default().fg(Theme::BRAND_PRIMARY)),
        Span::styled("  ", Style::default()),
        Span::styled("[ ◈ M365 Throttling ]", Style::default().fg(Theme::WARNING)),
        Span::styled("  ", Style::default()),
        Span::styled(profile_badge_text, profile_badge_style),
    ]);
    f.render_widget(Paragraph::new(badges_line).alignment(Alignment::Center), badges_area);

    // 3. MAIN MENU & CONTEXTUAL CARD
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
            Constraint::Percentage(50), // Interactive menu
            Constraint::Percentage(50), // Detail card
        ])
        .split(body_area);

    // Render Menu
    let menu_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(" Main Menu ");
    let menu_inner = menu_block.inner(body_chunks[0]);
    f.render_widget(menu_block, body_chunks[0]);

    let mut menu_lines = Vec::new();
    menu_lines.push(Line::from(""));

    for (idx, (title, _, _)) in MENU_ITEMS.iter().enumerate() {
        let is_selected = idx == state.welcome_menu_idx;
        let num_str = format!("[{}]", idx + 1);

        if is_selected {
            menu_lines.push(Line::from(vec![
                Span::styled(" ▸ ", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{}   ", num_str), Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!(" {:<14} ", title),
                    Style::default()
                        .bg(Theme::ACCENT_PRIMARY)
                        .fg(Theme::BG_DARK)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        } else {
            menu_lines.push(Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::styled(format!("{}   ", num_str), Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(
                    format!(" {:<14} ", title),
                    Style::default().fg(Theme::TEXT_MAIN),
                ),
            ]));
        }
        menu_lines.push(Line::from(""));
    }

    f.render_widget(Paragraph::new(menu_lines), menu_inner);

    // Render Contextual Card
    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_SECONDARY))
        .title(" Action Information ");
    let detail_inner = detail_block.inner(body_chunks[1]);
    f.render_widget(detail_block, body_chunks[1]);

    let current_menu = MENU_ITEMS[state.welcome_menu_idx.min(MENU_ITEMS.len() - 1)];
    let detail_lines = if state.welcome_menu_idx == 3 {
        let current_profile_desc = if state.use_default_profile {
            "• Active mode: Windows Default"
        } else if state.custom_profile_name.trim().is_empty() {
            "• Active mode: Manual (Name not assigned yet)"
        } else {
            "• Active mode: Custom manual profile"
        };
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Action: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled("MAPI Profile", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(Span::styled(current_profile_desc, Style::default().fg(Theme::TEXT_MAIN))),
            Line::from(""),
            Line::from(Span::styled(
                if !state.use_default_profile && !state.custom_profile_name.trim().is_empty() {
                    format!("• Assigned name: \"{}\"", state.custom_profile_name)
                } else {
                    "• Automatic connection to active system profile".to_string()
                },
                Style::default().fg(Theme::BRAND_PRIMARY),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("※ Tip: ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
                Span::styled("Press ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled("[Enter]", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled(" or ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled("[P]", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled(" to open the profile configuration dialog.", Style::default().fg(Theme::TEXT_MUTED)),
            ]),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Action: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(current_menu.0, Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(Span::styled(current_menu.1, Style::default().fg(Theme::TEXT_MAIN))),
            Line::from(""),
            Line::from(Span::styled(current_menu.2, Style::default().fg(Theme::TEXT_MUTED))),
            Line::from(""),
            Line::from(vec![
                Span::styled("※ Tip: ", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
                Span::styled("Press ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled("[Enter]", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled(" to execute or ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled("[1-5]", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled(" for quick access.", Style::default().fg(Theme::TEXT_MUTED)),
            ]),
        ]
    };
    f.render_widget(Paragraph::new(detail_lines), detail_inner);

    // 4. PROFILE EDITING MODAL (If active)
    if state.is_editing_profile {
        render_profile_modal(f, area, state);
    }
}

fn render_profile_modal(f: &mut Frame, area: Rect, state: &AppState) {
    let modal_width = 70.min(area.width.saturating_sub(4));
    let modal_height = 14.min(area.height.saturating_sub(2));

    let x = area.x + (area.width.saturating_sub(modal_width)) / 2;
    let y = area.y + (area.height.saturating_sub(modal_height)) / 2;
    let modal_rect = Rect::new(x, y, modal_width, modal_height);

    // Clear background beneath popup
    f.render_widget(Clear, modal_rect);

    let modal_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Theme::BRAND_PRIMARY))
        .title(" ◈ Configure Outlook MAPI Profile ");
    let inner = modal_block.inner(modal_rect);
    f.render_widget(modal_block, modal_rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Subtitle
            Constraint::Length(3), // Radio options
            Constraint::Length(3), // Textbox
            Constraint::Min(2),    // Action shortcuts
        ])
        .split(inner);

    // 1. Subtitle
    let sub = Paragraph::new(Line::from(Span::styled(
        "Select the MAPI profile containing the target mailboxes:",
        Style::default().fg(Theme::TEXT_MUTED),
    )));
    f.render_widget(sub, chunks[0]);

    // 2. Radio options
    let radio_1 = if state.use_default_profile {
        Span::styled("  [●] Windows Default Profile (Recommended)", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("  [○] Windows Default Profile", Style::default().fg(Theme::TEXT_MUTED))
    };
    let radio_2 = if !state.use_default_profile {
        Span::styled("  [●] Specify MAPI profile name manually:", Style::default().fg(Theme::BRAND_PRIMARY).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("  [○] Specify MAPI profile name manually", Style::default().fg(Theme::TEXT_MUTED))
    };

    let radio_p = Paragraph::new(vec![
        Line::from(radio_1),
        Line::from(radio_2),
    ]);
    f.render_widget(radio_p, chunks[1]);

    // 3. Textbox for custom_profile_name
    let input_border_color = if !state.use_default_profile {
        Theme::ACCENT_PRIMARY
    } else {
        Theme::TEXT_MUTED
    };

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(input_border_color))
        .title(" MAPI Profile Name ");

    let input_inner = input_block.inner(chunks[2]);
    f.render_widget(input_block, chunks[2]);

    let input_line = if state.use_default_profile {
        Line::from(Span::styled("  (Inactive — using Windows default profile)", Style::default().fg(Theme::TEXT_MUTED)))
    } else if state.custom_profile_name.is_empty() {
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("Type profile name...", Style::default().fg(Theme::WARNING)),
            Span::styled("█", Style::default().fg(Theme::BRAND_PRIMARY)),
        ])
    } else {
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(&state.custom_profile_name, Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
            Span::styled("█", Style::default().fg(Theme::BRAND_PRIMARY)),
        ])
    };
    f.render_widget(Paragraph::new(input_line), input_inner);

    // 4. Modal shortcuts
    let help_line = Line::from(vec![
        Span::styled("[Tab / ↑↓] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Mode   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[Type] ", Style::default().fg(Theme::ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled("Name   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[Enter] ", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
        Span::styled("Save   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[Esc] ", Style::default().fg(Theme::DANGER).add_modifier(Modifier::BOLD)),
        Span::styled("Close", Style::default().fg(Theme::TEXT_MUTED)),
    ]);
    f.render_widget(Paragraph::new(help_line).alignment(Alignment::Center), chunks[3]);
}
