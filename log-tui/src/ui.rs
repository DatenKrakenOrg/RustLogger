use crate::app::{App, Mode, SortDirection, SortField, IndexType, LogEntryType};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame,
};

/// Main UI rendering function that orchestrates drawing all TUI components.
///
/// This function handles the overall layout and determines which UI components
/// to render based on the current application mode. It creates a three-section
/// vertical layout (header, content, footer) for normal modes and switches to
/// a full-screen authentication interface when in Auth mode.
///
/// # Layout Structure
///
/// **Normal Modes:**
/// - Header (3 lines): Title, status, sort info, refresh indicators
/// - Content (flexible): Log list with syntax highlighting and selection
/// - Footer (3 lines): Context-sensitive help text
/// - Overlays: Input popups for search/limit, detail view for selected logs
///
/// **Auth Mode:**
/// - Full-screen authentication interface with centered input form
///
/// # Arguments
///
/// * `f` - Mutable reference to the terminal frame for rendering
/// * `app` - Mutable reference to the application state
pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.size());

    if app.mode == Mode::Auth {
        draw_auth_window(f, app);
    } else {
        draw_header(f, chunks[0], app);
        draw_logs(f, chunks[1], app);
        draw_footer(f, chunks[2], app);

        if app.mode == Mode::Search || app.mode == Mode::Limit {
            draw_input_popup(f, app);
        } else if app.mode == Mode::Details {
            draw_detail_popup(f, app);
        }
    }
}

/// Renders the header section with title, status, sort info, and refresh indicators.
///
/// The header displays context-sensitive information that varies by application mode
/// and current state. It includes loading indicators, error messages, auto-refresh
/// status, and sorting configuration.
///
/// # Header Elements
///
/// - **Title**: Current mode or index type name
/// - **Log count**: Current/limit display (e.g., "50/100 logs")
/// - **Sort info**: Active sort field and direction with arrows
/// - **Status**: Loading, error, or auto-refresh state
/// - **Last refresh**: Time elapsed since last data fetch
///
/// # Color Coding
///
/// - Title: Cyan with bold styling
/// - Log count: Green
/// - Sort info: Magenta
/// - Status: Yellow
/// - Last refresh: Light blue
/// - Errors: Displayed in status with error message
///
/// # Arguments
///
/// * `f` - Mutable reference to the terminal frame
/// * `area` - The rectangular area to render the header in
/// * `app` - Reference to the application state
fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let title = match app.mode {
        Mode::Auth => "Authentication",
        Mode::Normal => app.current_index_type.display_name(),
        Mode::Search => "Search Mode",
        Mode::Limit => "Limit Mode",
        Mode::Details => "Log Details",
    };

    let status_text = if app.loading {
        " [Loading...] ".to_string()
    } else if let Some(ref error) = app.error_message {
        format!(" [Error: {}] ", error)
    } else if app.auto_refresh {
        " [Auto-refresh ON] ".to_string()
    } else {
        " [Auto-refresh OFF] ".to_string()
    };

    let _last_refresh_text = format!(" | Last refresh: {}", 
        app.last_refresh.elapsed().as_secs() / 60,
    );
    let last_refresh_display = if app.last_refresh.elapsed().as_secs() < 60 {
        format!(" | Last refresh: {}s ago", app.last_refresh.elapsed().as_secs())
    } else {
        format!(" | Last refresh: {}m ago", app.last_refresh.elapsed().as_secs() / 60)
    };

    let sort_text = match app.current_index_type {
        IndexType::Logs => {
            format!("Sort: {} {}", 
                match app.sort_state.field {
                    SortField::Timestamp => "Time",
                    SortField::Level => "Level",
                    SortField::Device => "Device",
                    SortField::Temperature => "Temp",
                    SortField::Humidity => "Humid",
                },
                match app.sort_state.direction {
                    SortDirection::Ascending => "↑",
                    SortDirection::Descending => "↓",
                }
            )
        }
        IndexType::ContainerLogs => {
            format!("Sort: {} {}", 
                match app.sort_state.field {
                    SortField::Timestamp => "Time",
                    SortField::Device => "Container",
                    // For container logs, only Time and Container are valid
                    // If somehow we get other fields, default to Time but this shouldn't happen
                    _ => "Time",
                },
                match app.sort_state.direction {
                    SortDirection::Ascending => "↑",
                    SortDirection::Descending => "↓",
                }
            )
        }
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" | "),
        Span::styled(
            format!("{}/{} logs", app.logs.len(), app.log_limit),
            Style::default().fg(Color::Green),
        ),
        Span::raw(" | "),
        Span::styled(sort_text, Style::default().fg(Color::Magenta)),
        Span::styled(status_text, Style::default().fg(Color::Yellow)),
        Span::styled(last_refresh_display, Style::default().fg(Color::LightBlue)),
    ]))
    .block(Block::default().borders(Borders::ALL))
    .alignment(Alignment::Left);

    f.render_widget(header, area);
}

/// Renders the main log list view with syntax highlighting and selection.
///
/// This function displays the log entries in a scrollable list format with
/// different layouts for sensor logs vs container logs. It handles empty states,
/// selection highlighting, and search query display.
///
/// # Log Entry Formats
///
/// **Sensor Logs:**
/// `YYYY-MM-DD HH:MM:SS LEVEL    DEVICE          T:XX.X°C H:XX.X% Message`
///
/// **Container Logs:**
/// `YYYY-MM-DD HH:MM:SS CONTAINER_NAME       Log message`
///
/// # Visual Features
///
/// - **Timestamps**: Gray color for consistent visual hierarchy
/// - **Log levels**: Color-coded (Critical=Red, Warn=Yellow, Info=Blue)
/// - **Device/Container names**: Magenta for easy identification
/// - **Temperature/Humidity**: Blue for sensor data
/// - **Selection**: Dark gray background highlight
/// - **Search context**: Title shows active search query
///
/// # Empty States
///
/// - Loading: "Loading logs..."
/// - Error: "Failed to load logs. Press 'r' to retry."
/// - No data: "No logs found. Press 'r' to refresh."
///
/// # Arguments
///
/// * `f` - Mutable reference to the terminal frame
/// * `area` - The rectangular area to render the log list in
/// * `app` - Mutable reference to the application state
fn draw_logs(f: &mut Frame, area: Rect, app: &mut App) {
    if app.logs.is_empty() {
        let empty_msg = if app.loading {
            "Loading logs..."
        } else if app.error_message.is_some() {
            "Failed to load logs. Press 'r' to retry."
        } else {
            "No logs found. Press 'r' to refresh."
        };

        let paragraph = Paragraph::new(empty_msg)
            .block(Block::default().borders(Borders::ALL).title("Logs"))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = app
        .logs
        .iter()
        .enumerate()
        .map(|(i, log)| {
            let content = match log {
                LogEntryType::Regular(log_entry) => {
                    let level_color = app.get_log_level_color(&log_entry.level);
                    let timestamp = log_entry.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
                    let level_str = format!("{:?}", log_entry.level);
                    
                    Line::from(vec![
                        Span::styled(
                            format!("{:<19}", timestamp),
                            Style::default().fg(Color::Gray),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("{:<8}", level_str),
                            Style::default().fg(level_color).add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("{:<15}", log_entry.msg.device),
                            Style::default().fg(Color::Magenta),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("T:{:.1}°C H:{:.1}% ",
                                log_entry.temperature,
                                log_entry.humidity
                            ),
                            Style::default().fg(Color::Blue),
                        ),
                        Span::raw(log_entry.msg.msg.clone()),
                    ])
                }
                LogEntryType::Container(log_entry) => {
                    let timestamp = log_entry.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
                    
                    Line::from(vec![
                        Span::styled(
                            format!("{:<19}", timestamp),
                            Style::default().fg(Color::Gray),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("{:<20}", log_entry.container_name),
                            Style::default().fg(Color::Magenta),
                        ),
                        Span::raw(" "),
                        Span::raw(log_entry.log_message.clone()),
                    ])
                }
            };

            let style = if i == app.selected_index {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let title = if !app.search_query.is_empty() {
        format!("Logs (Search: '{}')", app.search_query)
    } else {
        "Logs".to_string()
    };

    let logs_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().bg(Color::DarkGray));

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected_index));

    f.render_stateful_widget(logs_list, area, &mut list_state);
}

/// Renders the footer with context-sensitive help text based on current mode.
///
/// The footer displays keyboard shortcuts and instructions that change
/// dynamically based on the current application mode, providing users
/// with relevant guidance for available actions.
///
/// # Mode-specific Help Text
///
/// - **Auth**: API key entry and authentication instructions
/// - **Normal**: Full navigation, sorting, and action shortcuts
/// - **Search**: Search query input and execution instructions
/// - **Limit**: Log limit setting instructions with current value
/// - **Details**: Detail view navigation instructions
///
/// # Styling
///
/// - Gray text for unobtrusive help information
/// - Centered alignment for balanced layout
/// - Text wrapping for responsive display
///
/// # Arguments
///
/// * `f` - Mutable reference to the terminal frame
/// * `area` - The rectangular area to render the footer in
/// * `app` - Reference to the application state
fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let help_text = match app.mode {
        Mode::Auth => {
            "Enter your API key | Enter: Authenticate | q: Quit"
        }
        Mode::Normal => {
            "↑/↓: Navigate | Enter: Details | /: Search | f: Sort field | o: Sort order | l: Limit | r: Refresh | a: Auto-refresh | c: Clear | i: Switch index | q: Quit"
        }
        Mode::Search => {
            "Type search query | Enter: Execute search | Esc: Cancel"
        }
        Mode::Limit => {
            "Enter number of logs to fetch (current: {}) | Enter: Apply | Esc: Cancel"
        }
        Mode::Details => {
            "Enter/Esc: Close details"
        }
    };
    
    let help_text = if app.mode == Mode::Limit {
        help_text.replace("{}", &app.log_limit.to_string())
    } else {
        help_text.to_string()
    };

    let footer = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(footer, area);
}

/// Renders a modal input dialog for search and limit entry.
///
/// This function creates a centered popup overlay for user input in Search
/// and Limit modes. The popup includes a text input field with cursor
/// positioning and appropriate titles based on the mode.
///
/// # Input Modes
///
/// - **Search Mode**: "Search Logs" - for entering search queries
/// - **Limit Mode**: "Set Log Limit" - for entering log count limits
///
/// # Visual Features
///
/// - Centered overlay (60% width, 20% height)
/// - Yellow text for input visibility
/// - Bordered container with mode-specific title
/// - Live cursor positioning at input end
/// - Clears background content behind popup
///
/// # Arguments
///
/// * `f` - Mutable reference to the terminal frame
/// * `app` - Reference to the application state
fn draw_input_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.size());
    f.render_widget(Clear, area);

    let title = match app.mode {
        Mode::Search => "Search Logs",
        Mode::Limit => "Set Log Limit",
        _ => "Input",
    };

    let input = Paragraph::new(app.input_buffer.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(input, area);

    f.set_cursor(
        area.x + app.input_buffer.len() as u16 + 1,
        area.y + 1,
    );
}

/// Renders a detailed view popup for the selected log entry.
///
/// This function displays a comprehensive view of the currently selected log
/// entry in a large modal popup. The content and format vary between sensor
/// logs and container logs to show all relevant information.
///
/// # Sensor Log Details
///
/// - **Timestamp**: Full date/time with UTC designation
/// - **Level**: Color-coded log level (Critical/Warn/Info)
/// - **Device**: Source device name in magenta
/// - **Temperature**: Precise temperature reading in °C
/// - **Humidity**: Precise humidity percentage
/// - **Message**: Full log message content
/// - **Exceeded Values**: Boolean flag for threshold violations
///
/// # Container Log Details
///
/// - **Timestamp**: Full date/time with UTC designation
/// - **Container**: Container name in magenta
/// - **Message**: Full log message content
///
/// # Visual Features
///
/// - Large centered overlay (80% width, 50% height)
/// - Bordered container with "Log Details" title
/// - Bold field labels for easy scanning
/// - Color-coded values matching main list view
/// - Text wrapping for long content
/// - Clears background content behind popup
///
/// # Arguments
///
/// * `f` - Mutable reference to the terminal frame
/// * `app` - Reference to the application state
fn draw_detail_popup(f: &mut Frame, app: &App) {
    if let Some(log) = app.get_selected_log() {
        let area = centered_rect(80, 50, f.size());
        f.render_widget(Clear, area);

        let content = match log {
            LogEntryType::Regular(log_entry) => {
                let timestamp = log_entry.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string();
                let level_str = format!("{:?}", log_entry.level);
                let level_color = app.get_log_level_color(&log_entry.level);

                Text::from(vec![
                    Line::from(vec![
                        Span::styled("Timestamp: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(timestamp),
                    ]),
                    Line::from(vec![
                        Span::styled("Level: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(level_str, Style::default().fg(level_color)),
                    ]),
                    Line::from(vec![
                        Span::styled("Device: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(log_entry.msg.device.clone(), Style::default().fg(Color::Magenta)),
                    ]),
                    Line::from(vec![
                        Span::styled("Temperature: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(format!("{:.2}°C", log_entry.temperature), Style::default().fg(Color::Blue)),
                    ]),
                    Line::from(vec![
                        Span::styled("Humidity: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(format!("{:.2}%", log_entry.humidity), Style::default().fg(Color::Blue)),
                    ]),
                    Line::from(vec![
                        Span::styled("Message: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(log_entry.msg.msg.clone()),
                    ]),
                    Line::from(vec![
                        Span::styled("Exceeded Values: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(format!("{:?}", log_entry.msg.exceeded_values)),
                    ]),
                ])
            }
            LogEntryType::Container(log_entry) => {
                let timestamp = log_entry.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string();

                Text::from(vec![
                    Line::from(vec![
                        Span::styled("Timestamp: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(timestamp),
                    ]),
                    Line::from(vec![
                        Span::styled("Container: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(log_entry.container_name.clone(), Style::default().fg(Color::Magenta)),
                    ]),
                    Line::from(vec![
                        Span::styled("Message: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(log_entry.log_message.clone()),
                    ]),
                ])
            }
        };

        let detail = Paragraph::new(content)
            .block(Block::default().borders(Borders::ALL).title("Log Details"))
            .wrap(Wrap { trim: true });

        f.render_widget(detail, area);
    }
}

/// Renders the full-screen authentication interface for API key entry.
///
/// This function creates a centered authentication form that takes over the
/// entire screen when the application is in Auth mode. It provides a secure
/// interface for entering API keys with visual feedback for different states.
///
/// # Layout Structure
///
/// The interface uses a centered layout with:
/// - **Title section**: "Log Viewer Authentication" with cyan styling
/// - **Input section**: Masked API key input field
/// - **Status section**: Error messages, loading indicators, or instructions
/// - **Footer**: Keyboard shortcuts for authentication actions
///
/// # Security Features
///
/// - **Input masking**: API key characters are hidden with asterisks
/// - **Visual states**: Different colors for normal, loading, and error states
/// - **Loading feedback**: "Authenticating..." message during API validation
///
/// # Visual States
///
/// - **Normal**: Green input field with gray instructions
/// - **Loading**: Yellow text with "Authenticating..." message
/// - **Error**: Red error message display
/// - **Success**: Transitions to normal mode automatically
///
/// # Responsive Design
///
/// - Centered horizontally and vertically
/// - 60% width for comfortable input area
/// - Flexible height based on content
/// - Cursor positioning for active input
///
/// # Arguments
///
/// * `f` - Mutable reference to the terminal frame
/// * `app` - Reference to the application state
fn draw_auth_window(f: &mut Frame, app: &App) {
    let area = f.size();
    
    // Create a centered layout for the authentication form
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Min(10),
            Constraint::Percentage(30),
        ])
        .split(area);

    let auth_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(chunks[1]);

    let content_area = auth_chunks[1];

    // Split content area for title, input, and status
    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(content_area);

    // Draw title
    let title = Paragraph::new("Log Viewer Authentication")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(title, content_chunks[0]);

    // Draw API key input (masked)
    let input_text = if app.loading {
        "Authenticating...".to_string()
    } else {
        app.get_masked_input()
    };

    let input = Paragraph::new(input_text.as_str())
        .style(if app.loading {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Green)
        })
        .block(Block::default().borders(Borders::ALL).title("API Key"));
    f.render_widget(input, content_chunks[1]);

    // Set cursor position if not loading
    if !app.loading {
        f.set_cursor(
            content_chunks[1].x + app.input_buffer.len() as u16 + 1,
            content_chunks[1].y + 1,
        );
    }

    // Draw status/error message
    if let Some(ref error) = app.auth_error {
        let error_msg = Paragraph::new(error.as_str())
            .style(Style::default().fg(Color::Red))
            .block(Block::default().borders(Borders::ALL).title("Error"))
            .alignment(Alignment::Center);
        f.render_widget(error_msg, content_chunks[2]);
    } else if app.loading {
        let loading_msg = Paragraph::new("Please wait while authenticating...")
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .alignment(Alignment::Center);
        f.render_widget(loading_msg, content_chunks[2]);
    } else {
        let help_msg = Paragraph::new("Enter your API key and press Enter to authenticate")
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title("Instructions"))
            .alignment(Alignment::Center);
        f.render_widget(help_msg, content_chunks[2]);
    }

    // Draw footer with help text
    let footer_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area)[1];

    let footer = Paragraph::new("Enter: Authenticate | q: Quit")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(footer, footer_area);
}

/// Helper function to calculate centered rectangular areas for popups and modals.
///
/// This utility function creates a centered rectangle within a given area based
/// on percentage dimensions. It's commonly used for positioning modal dialogs,
/// popups, and other overlay components in the center of the screen.
///
/// # Calculation Method
///
/// The function uses a three-section layout in both directions:
/// - **Margins**: `(100 - percent) / 2` for top/bottom and left/right
/// - **Content**: `percent` for the actual popup area
/// - **Margins**: `(100 - percent) / 2` for remaining space
///
/// # Arguments
///
/// * `percent_x` - Width percentage of the parent area (0-100)
/// * `percent_y` - Height percentage of the parent area (0-100)
/// * `r` - The parent rectangular area to center within
///
/// # Returns
///
/// A `Rect` representing the centered area with the specified dimensions
///
/// # Example
///
/// ```rust
/// // Create a popup that's 60% width and 40% height, centered on screen
/// let popup_area = centered_rect(60, 40, terminal_size);
/// ```
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
