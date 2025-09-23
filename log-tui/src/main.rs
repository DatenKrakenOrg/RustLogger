mod api;
mod app;
mod ui;

use app::{App, Mode};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::{
    env,
    error::Error,
    io,
    time::{Duration, Instant},
};


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let api_base_url = env::var("LOG_API_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(api_base_url);

    let res = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

/// Runs the main application event loop for the terminal UI.
///
/// This function handles the core TUI lifecycle including:
/// - Rendering the terminal interface at regular intervals (250ms tick rate)
/// - Processing user input events (keyboard)
/// - Managing auto-refresh functionality for log data
/// - Coordinating between different application modes (Auth, Normal, Search, Details, Limit)
///
/// # Arguments
///
/// * `terminal` - Mutable reference to the terminal backend for rendering
/// * `app` - Mutable reference to the application state
///
/// # Returns
///
/// Returns `Ok(())` on successful exit, or an `io::Error` if terminal operations fail
///
/// # Event Loop
///
/// The loop runs at 250ms intervals and handles:
/// - Terminal drawing via `ui::draw`
/// - Input polling with timeout
/// - Auto-refresh when enabled and not in Auth mode
/// - Mode-specific keyboard shortcuts and navigation
///
/// # Keyboard Controls
///
/// **Auth Mode:**
/// - `q` - Quit application
/// - `Enter` - Submit API key
/// - `Backspace` - Delete character
/// - Characters - Input API key
///
/// **Normal Mode:**
/// - `q` - Quit application
/// - `Up/Down` - Navigate log entries
/// - `r` - Manual refresh
/// - `/` - Enter search mode
/// - `f` - Cycle sort field
/// - `o` - Toggle sort direction
/// - `l` - Enter limit mode
/// - `a` - Toggle auto-refresh
/// - `c` - Clear search
/// - `i` - Switch between sensor/container logs
/// - `Enter` - View log details
///
/// **Details Mode:**
/// - `Esc/Enter` - Exit details view
///
/// **Search/Limit Mode:**
/// - `Enter` - Execute search/limit
/// - `Esc` - Cancel input
/// - `Backspace` - Delete character
/// - Characters - Input text/numbers
async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(250);

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        let timeout_duration = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout_duration)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                match app.mode {
                    Mode::Auth => {
                        match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Enter => {
                                if let Err(e) = app.execute_input().await {
                                    app.auth_error = Some(format!("Authentication failed: {}", e));
                                }
                            }
                            KeyCode::Char(c) => {
                                app.handle_input_char(c);
                            }
                            KeyCode::Backspace => {
                                app.handle_backspace();
                            }
                            _ => {}
                        }
                    }
                    Mode::Normal => {
                        match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Up => app.move_selection_up(),
                            KeyCode::Down => app.move_selection_down(),
                            KeyCode::Char('r') => {
                                if let Err(e) = app.refresh_logs().await {
                                    app.error_message = Some(format!("Refresh failed: {}", e));
                                }
                            }
                            KeyCode::Char('/') => {
                                app.enter_search_mode();
                            }
                             KeyCode::Char('f') => {
                                 app.cycle_sort_field();
                             }
                             KeyCode::Char('o') => {
                                 app.toggle_sort_direction();
                             }
                             KeyCode::Char('l') => {
                                 app.enter_limit_mode();
                             }
                            KeyCode::Char('a') => {
                                app.toggle_auto_refresh();
                            }
                             KeyCode::Char('c') => {
                                app.clear_search();
                                if let Err(e) = app.refresh_logs().await {
                                    app.error_message = Some(format!("Refresh failed: {}", e));
                                }
                            }
                             KeyCode::Char('i') => {
                                 app.switch_index();
                                 if let Err(e) = app.refresh_logs().await {
                                     app.error_message = Some(format!("Refresh failed: {}", e));
                                 }
                             }
                             KeyCode::Enter => {
                                 app.enter_details_mode();
                             }
                            _ => {}
                        }
                    }
                        Mode::Details => {
                            match key.code {
                                KeyCode::Esc | KeyCode::Enter => {
                                    app.exit_mode();
                                }
                                _ => {}
                            }
                        }
                    Mode::Search | Mode::Limit => {
                        match key.code {
                             KeyCode::Enter => {
                                 if let Err(e) = app.execute_input().await {
                                     app.error_message = Some(format!("Input failed: {}", e));
                                     app.exit_mode();
                                 }
                             }
                            KeyCode::Esc => {
                                app.exit_mode();
                            }
                            KeyCode::Char(c) => {
                                app.handle_input_char(c);
                            }
                            KeyCode::Backspace => {
                                app.handle_backspace();
                            }
                            _ => {}
                        }
                    }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            if app.mode != Mode::Auth && app.should_refresh() {
                if let Err(e) = app.refresh_logs().await {
                    app.error_message = Some(format!("Auto-refresh failed: {}", e));
                }
            }
            last_tick = Instant::now();
        }
    }
}
