mod app;
mod ui;

use std::{io, time::Duration};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, Terminal,
};

use app::{AppState, WizardStep};
use ui::{footer::render_footer, header::render_header, theme::Theme};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configuración inicial de la terminal en modo Raw y pantalla alternativa
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = AppState::new();

    // Event Loop Reactivo de Ultra Bajo Overhead (Sin busy-loop)
    while !state.should_quit {
        terminal.draw(|f| draw_ui(f, &state))?;

        // Espera de eventos con timeout de 50ms para mantener la CPU < 1% en reposo
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                // Captura universal de salida inmediata
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    if state.step == WizardStep::Execution {
                        state.progress.graceful_cancelling = true;
                        state.log_event("[SISTEMA] Cancelación recibida. Desmontando PST de forma segura...".to_string());
                    } else {
                        state.should_quit = true;
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') => {
                        if state.step != WizardStep::Execution {
                            state.should_quit = true;
                        }
                    }
                    KeyCode::Enter => {
                        if state.step == WizardStep::Completion {
                            state.should_quit = true;
                        } else {
                            state.next_step();
                        }
                    }
                    KeyCode::Esc | KeyCode::Backspace => {
                        if state.step == WizardStep::Execution {
                            state.progress.graceful_cancelling = true;
                            state.log_event("[SISTEMA] Cancelación solicitada por usuario. Parada segura en curso...".to_string());
                        } else {
                            state.prev_step();
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Restaurar la terminal limpia
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn draw_ui(f: &mut Frame, state: &AppState) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header superior
            Constraint::Min(14),    // Pantalla activa del wizard
            Constraint::Length(3), // Footer inferior con marca de agua
        ])
        .split(size);

    // 1. Renderizar Header
    render_header(
        f,
        chunks[0],
        state.step.title(),
        state.step.index(),
        8,
    );

    // 2. Renderizar Contenido Central según el Paso
    let content_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ACCENT_PRIMARY))
        .title(Span::styled(
            format!(" {} ", state.step.title()),
            Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD),
        ));

    let inner = content_block.inner(chunks[1]);
    f.render_widget(content_block, chunks[1]);

    let placeholder_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Paso Activo: ", Style::default().fg(Theme::ACCENT_PRIMARY)),
            Span::styled(state.step.title(), Style::default().fg(Theme::TEXT_MAIN).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Presiona [Enter] para avanzar al siguiente paso, [Esc] para retroceder, o [q] para salir.",
            Style::default().fg(Theme::TEXT_MUTED),
        )),
    ];
    let p = Paragraph::new(placeholder_text).alignment(Alignment::Center);
    f.render_widget(p, inner);

    // 3. Renderizar Footer con atajos y marca de agua Timeless Support
    let shortcuts = match state.step {
        WizardStep::Execution => vec![("Esc/Ctrl+C", "Parada Segura")],
        WizardStep::Completion => vec![("Enter/q", "Salir")],
        _ => vec![
            ("Enter", "Siguiente"),
            ("Esc", "Atrás"),
            ("q", "Salir"),
        ],
    };
    render_footer(f, chunks[2], &shortcuts);
}
