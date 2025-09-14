use crate::api::{ApiClient, LogEntry, LogLevel, ContainerLogEntry};
use anyhow::Result;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Auth,
    Normal,
    Search,
    Limit,
    Details,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IndexType {
    Logs,
    ContainerLogs,
}

impl IndexType {
    pub fn display_name(&self) -> &str {
        match self {
            IndexType::Logs => "Sensor Logs",
            IndexType::ContainerLogs => "Container Logs",
        }
    }
}

#[derive(Debug, Clone)]
pub enum LogEntryType {
    Regular(LogEntry),
    Container(ContainerLogEntry),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortField {
    Timestamp,
    Level,
    Device,
    Temperature,
    Humidity,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone)]
pub struct SortState {
    pub field: SortField,
    pub direction: SortDirection,
}

impl Default for SortState {
    fn default() -> Self {
        Self {
            field: SortField::Timestamp,
            direction: SortDirection::Descending,
        }
    }
}

pub struct App {
    pub logs: Vec<LogEntryType>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub mode: Mode,
    pub current_index_type: IndexType,
    pub search_query: String,
    pub sort_state: SortState,
    pub log_limit: usize,
    pub input_buffer: String,
    pub api_client: ApiClient,
    pub last_refresh: Instant,
    pub auto_refresh: bool,
    pub refresh_interval: Duration,
    pub loading: bool,
    pub error_message: Option<String>,
    pub api_key: Option<String>,
    pub auth_error: Option<String>,
}

impl App {
    pub fn new(api_base_url: String) -> Self {
        Self {
            logs: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            mode: Mode::Auth,
            current_index_type: IndexType::Logs,
            search_query: String::new(),
            sort_state: SortState::default(),
            log_limit: 100,
            input_buffer: String::new(),
            api_client: ApiClient::new(api_base_url),
            last_refresh: Instant::now(),
            auto_refresh: true,
            refresh_interval: Duration::from_secs(5),
            loading: false,
            error_message: None,
            api_key: None,
            auth_error: None,
        }
    }

    pub fn should_refresh(&self) -> bool {
        self.auto_refresh && self.last_refresh.elapsed() >= self.refresh_interval
    }

    pub async fn refresh_logs(&mut self) -> Result<()> {
        self.loading = true;
        self.error_message = None;

        let result = match self.current_index_type {
            IndexType::Logs => {
                if !self.search_query.is_empty() {
                    self.api_client
                        .search_logs(&self.search_query, Some(self.log_limit), Some(0))
                        .await
                        .map(|logs| logs.into_iter().map(LogEntryType::Regular).collect())
                } else {
                    self.api_client
                        .fetch_logs(Some(self.log_limit), Some(0), None, None, None, None)
                        .await
                        .map(|logs| logs.into_iter().map(LogEntryType::Regular).collect())
                }
            }
            IndexType::ContainerLogs => {
                if !self.search_query.is_empty() {
                    self.api_client
                        .search_container_logs(&self.search_query, Some(self.log_limit), Some(0))
                        .await
                        .map(|logs| logs.into_iter().map(LogEntryType::Container).collect())
                } else {
                    self.api_client
                        .fetch_container_logs(Some(self.log_limit), Some(0), None, None, None)
                        .await
                        .map(|logs| logs.into_iter().map(LogEntryType::Container).collect())
                }
            }
        };

        match result {
            Ok(mut logs) => {
                self.sort_logs(&mut logs);
                self.logs = logs;
                self.last_refresh = Instant::now();
                if self.selected_index >= self.logs.len() && !self.logs.is_empty() {
                    self.selected_index = self.logs.len() - 1;
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to fetch logs: {}", e));
            }
        }

        self.loading = false;
        Ok(())
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            }
        }
    }

    pub fn move_selection_down(&mut self) {
        if self.selected_index + 1 < self.logs.len() {
            self.selected_index += 1;
        }
    }

    pub fn enter_search_mode(&mut self) {
        self.mode = Mode::Search;
        self.input_buffer.clear();
    }

    pub fn enter_limit_mode(&mut self) {
        self.mode = Mode::Limit;
        self.input_buffer = self.log_limit.to_string();
    }

    pub fn exit_mode(&mut self) {
        self.mode = Mode::Normal;
        self.input_buffer.clear();
    }

    pub fn handle_input_char(&mut self, c: char) {
        self.input_buffer.push(c);
    }

    pub fn handle_backspace(&mut self) {
        self.input_buffer.pop();
    }

    pub async fn execute_input(&mut self) -> Result<()> {
        match self.mode {
            Mode::Search => {
                self.search_query = self.input_buffer.clone();
                self.mode = Mode::Normal;
                self.input_buffer.clear();
                self.refresh_logs().await
            }
            Mode::Limit => {
                if let Ok(limit) = self.input_buffer.parse::<usize>() {
                    self.log_limit = limit.max(1);
                }
                self.mode = Mode::Normal;
                self.input_buffer.clear();
                self.refresh_logs().await
            }
            Mode::Auth => {
                self.authenticate().await
            }
            _ => Ok(())
        }
    }
    
    pub fn sort_logs(&self, logs: &mut Vec<LogEntryType>) {
        match self.current_index_type {
            IndexType::Logs => {
                logs.sort_by(|a, b| {
                    if let (LogEntryType::Regular(a), LogEntryType::Regular(b)) = (a, b) {
                        let cmp = match self.sort_state.field {
                            SortField::Timestamp => a.timestamp.cmp(&b.timestamp),
                            SortField::Level => {
                                let a_priority = match a.level {
                                    LogLevel::Critical => 2,
                                    LogLevel::Warn => 1,
                                    LogLevel::Info => 0,
                                };
                                let b_priority = match b.level {
                                    LogLevel::Critical => 2,
                                    LogLevel::Warn => 1,
                                    LogLevel::Info => 0,
                                };
                                a_priority.cmp(&b_priority)
                            }
                            SortField::Device => a.msg.device.cmp(&b.msg.device),
                            SortField::Temperature => a.temperature.partial_cmp(&b.temperature).unwrap_or(std::cmp::Ordering::Equal),
                            SortField::Humidity => a.humidity.partial_cmp(&b.humidity).unwrap_or(std::cmp::Ordering::Equal),
                        };

                        match self.sort_state.direction {
                            SortDirection::Ascending => cmp,
                            SortDirection::Descending => cmp.reverse(),
                        }
                    } else {
                        std::cmp::Ordering::Equal
                    }
                });
            }
            IndexType::ContainerLogs => {
                logs.sort_by(|a, b| {
                    if let (LogEntryType::Container(a), LogEntryType::Container(b)) = (a, b) {
                        let cmp = match self.sort_state.field {
                            SortField::Timestamp => a.timestamp.cmp(&b.timestamp),
                            SortField::Device => a.container_name.cmp(&b.container_name), // Use container_name as "device"
                            _ => a.timestamp.cmp(&b.timestamp), // Default to timestamp for other fields
                        };

                        match self.sort_state.direction {
                            SortDirection::Ascending => cmp,
                            SortDirection::Descending => cmp.reverse(),
                        }
                    } else {
                        std::cmp::Ordering::Equal
                    }
                });
            }
        }
    }
    
    pub fn apply_current_sort(&mut self) {
        let sort_field = self.sort_state.field;
        let sort_direction = self.sort_state.direction;
        let index_type = self.current_index_type;
        
        match index_type {
            IndexType::Logs => {
                self.logs.sort_by(|a, b| {
                    if let (LogEntryType::Regular(a), LogEntryType::Regular(b)) = (a, b) {
                        let cmp = match sort_field {
                            SortField::Timestamp => a.timestamp.cmp(&b.timestamp),
                            SortField::Level => {
                                let a_priority = match a.level {
                                    LogLevel::Critical => 2,
                                    LogLevel::Warn => 1,
                                    LogLevel::Info => 0,
                                };
                                let b_priority = match b.level {
                                    LogLevel::Critical => 2,
                                    LogLevel::Warn => 1,
                                    LogLevel::Info => 0,
                                };
                                a_priority.cmp(&b_priority)
                            }
                            SortField::Device => a.msg.device.cmp(&b.msg.device),
                            SortField::Temperature => a.temperature.partial_cmp(&b.temperature).unwrap_or(std::cmp::Ordering::Equal),
                            SortField::Humidity => a.humidity.partial_cmp(&b.humidity).unwrap_or(std::cmp::Ordering::Equal),
                        };

                        match sort_direction {
                            SortDirection::Ascending => cmp,
                            SortDirection::Descending => cmp.reverse(),
                        }
                    } else {
                        std::cmp::Ordering::Equal
                    }
                });
            }
            IndexType::ContainerLogs => {
                self.logs.sort_by(|a, b| {
                    if let (LogEntryType::Container(a), LogEntryType::Container(b)) = (a, b) {
                        let cmp = match sort_field {
                            SortField::Timestamp => a.timestamp.cmp(&b.timestamp),
                            SortField::Device => a.container_name.cmp(&b.container_name),
                            _ => a.timestamp.cmp(&b.timestamp),
                        };

                        match sort_direction {
                            SortDirection::Ascending => cmp,
                            SortDirection::Descending => cmp.reverse(),
                        }
                    } else {
                        std::cmp::Ordering::Equal
                    }
                });
            }
        }
        
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.mode = Mode::Normal;
    }
    
    pub fn cycle_sort_field(&mut self) {
        self.sort_state.field = match self.current_index_type {
            IndexType::Logs => {
                // For sensor logs, cycle through all available fields
                match self.sort_state.field {
                    SortField::Timestamp => SortField::Level,
                    SortField::Level => SortField::Device,
                    SortField::Device => SortField::Temperature,
                    SortField::Temperature => SortField::Humidity,
                    SortField::Humidity => SortField::Timestamp,
                }
            }
            IndexType::ContainerLogs => {
                // For container logs, only cycle between Timestamp and Device (container name)
                match self.sort_state.field {
                    SortField::Timestamp => SortField::Device,
                    _ => SortField::Timestamp, // Any other field goes back to timestamp
                }
            }
        };
        self.apply_current_sort();
    }
    
    pub fn toggle_sort_direction(&mut self) {
        self.sort_state.direction = match self.sort_state.direction {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        };
        self.apply_current_sort();
    }
    
    pub fn enter_details_mode(&mut self) {
        if !self.logs.is_empty() {
            self.mode = Mode::Details;
        }
    }

    pub fn toggle_auto_refresh(&mut self) {
        self.auto_refresh = !self.auto_refresh;
    }

    pub fn get_selected_log(&self) -> Option<&LogEntryType> {
        self.logs.get(self.selected_index)
    }

    pub fn get_log_level_color(&self, level: &LogLevel) -> ratatui::style::Color {
        match level {
            LogLevel::Critical => ratatui::style::Color::Red,
            LogLevel::Warn => ratatui::style::Color::Yellow,
            LogLevel::Info => ratatui::style::Color::Blue,
        }
    }

    pub async fn authenticate(&mut self) -> Result<()> {
        if self.input_buffer.trim().is_empty() {
            self.auth_error = Some("API key cannot be empty".to_string());
            return Ok(());
        }

        self.loading = true;
        self.auth_error = None;
        
        let api_key = self.input_buffer.trim().to_string();
        self.api_client.set_api_key(Some(api_key.clone()));
        
        // Test the API key by making a simple request
        match self.api_client.fetch_logs(Some(1), Some(0), None, None, None, None).await {
            Ok(_) => {
                self.api_key = Some(api_key);
                self.mode = Mode::Normal;
                self.input_buffer.clear();
                self.loading = false;
                // Fetch initial logs
                self.refresh_logs().await
            }
            Err(e) => {
                self.loading = false;
                self.auth_error = Some(format!("Authentication failed: {}", e));
                self.api_client.set_api_key(None);
                Ok(())
            }
        }
    }

    pub fn switch_index(&mut self) {
        self.current_index_type = match self.current_index_type {
            IndexType::Logs => IndexType::ContainerLogs,
            IndexType::ContainerLogs => IndexType::Logs,
        };
        
        // Reset sort field to a valid one for the new index type
        match self.current_index_type {
            IndexType::ContainerLogs => {
                // For container logs, ensure we're using a valid sort field
                if !matches!(self.sort_state.field, SortField::Timestamp | SortField::Device) {
                    self.sort_state.field = SortField::Timestamp;
                }
            }
            IndexType::Logs => {
                // For sensor logs, all fields are valid, so no need to reset
            }
        }
        
        // Clear current logs and reset selection
        self.logs.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.search_query.clear();
        self.error_message = None;
    }



    pub fn get_masked_input(&self) -> String {
        "*".repeat(self.input_buffer.len())
    }
}
