use crate::app::{App, Mode, SortDirection, SortField};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.size());

    draw_header(f, chunks[0], app);
    draw_logs(f, chunks[1], app);
    draw_footer(f, chunks[2], app);

    if app.mode == Mode::Search || app.mode == Mode::Sort || app.mode == Mode::Limit {
        draw_input_popup(f, app);
    } else if app.mode == Mode::Details {
        draw_detail_popup(f, app);
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let title = match app.mode {
        Mode::Normal => "Log Viewer",
        Mode::Search => "Search Mode",
        Mode::Sort => "Sort Mode",
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

    let header = Paragraph::new(Line::from(vec![
        Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" | "),
        Span::styled(
            format!("{}/{} logs", app.logs.len(), app.log_limit),
            Style::default().fg(Color::Green),
        ),
        Span::raw(" | "),
        Span::styled(
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
            ),
            Style::default().fg(Color::Magenta),
        ),
        Span::styled(status_text, Style::default().fg(Color::Yellow)),
    ]))
    .block(Block::default().borders(Borders::ALL))
    .alignment(Alignment::Left);

    f.render_widget(header, area);
}

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
            let level_color = app.get_log_level_color(&log.level);
            let timestamp = log.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
            let level_str = format!("{:?}", log.level);
            
            let content = Line::from(vec![
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
                    format!("{:<15}", log.msg.device),
                    Style::default().fg(Color::Magenta),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("T:{:.1}°C H:{:.1}% ",
                        log.temperature,
                        log.humidity
                    ),
                    Style::default().fg(Color::Blue),
                ),
                Span::raw(log.msg.msg.clone()),
            ]);

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

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let help_text = match app.mode {
        Mode::Normal => {
            "↑/↓: Navigate | w/s: Page | Enter: Details | /: Search | f: Sort field | o: Sort order | S: Custom sort | l: Limit | r: Refresh | a: Auto-refresh | c: Clear | q: Quit"
        }
        Mode::Search => {
            "Type search query | Enter: Execute search | Esc: Cancel"
        }
        Mode::Sort => {
            "Sort commands: 'timestamp asc', 'level desc', 'device', 'temperature', 'humidity' | Enter: Apply | Esc: Cancel"
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

fn draw_input_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.size());
    f.render_widget(Clear, area);

    let title = match app.mode {
        Mode::Search => "Search Logs",
        Mode::Sort => "Sort Logs (field [asc|desc])",
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

fn draw_detail_popup(f: &mut Frame, app: &App) {
    if let Some(log) = app.get_selected_log() {
        let area = centered_rect(80, 50, f.size());
        f.render_widget(Clear, area);

        let timestamp = log.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let level_str = format!("{:?}", log.level);
        let level_color = app.get_log_level_color(&log.level);

        let content = Text::from(vec![
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
                Span::styled(log.msg.device.clone(), Style::default().fg(Color::Magenta)),
            ]),
            Line::from(vec![
                Span::styled("Temperature: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:.2}°C", log.temperature), Style::default().fg(Color::Blue)),
            ]),
            Line::from(vec![
                Span::styled("Humidity: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:.2}%", log.humidity), Style::default().fg(Color::Blue)),
            ]),
            Line::from(vec![
                Span::styled("Message: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(log.msg.msg.clone()),
            ]),
            Line::from(vec![
                Span::styled("Exceeded Values: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{:?}", log.msg.exceeded_values)),
            ]),
        ]);

        let detail = Paragraph::new(content)
            .block(Block::default().borders(Borders::ALL).title("Log Details"))
            .wrap(Wrap { trim: true });

        f.render_widget(detail, area);
    }
}

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