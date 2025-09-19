# Log TUI

A terminal user interface for viewing logs from the log-forwarding-API.

## Features

- Real-time log viewing with auto-refresh
- Search functionality across log messages
- Sort logs by timestamp, level, device, temperature, or humidity
- Configurable log limit (fetch more or fewer logs)
- Keyboard navigation and shortcuts
- Color-coded log levels

## Usage

### Environment Variables

- `LOG_API_URL`: Base URL for the log-forwarding API (default: http://localhost:8080)

### Running

```bash
# Set the API URL (optional)
export LOG_API_URL=http://localhost:8080

# Run the TUI
cargo run
```

### Keyboard Shortcuts

#### Normal Mode
- `↑/↓`: Navigate up/down through logs
- `w/s`: Page up/down through logs
- `Enter`: View log details
- `/`: Enter search mode
- `f`: Cycle through sort fields (timestamp → level → device → temperature → humidity)
- `o`: Toggle sort order (ascending ↔ descending)
- `S`: Enter custom sort mode (advanced sorting)
- `l`: Set log limit (number of logs to fetch)
- `r`: Manually refresh logs
- `a`: Toggle auto-refresh on/off
- `c`: Clear current search
- `q`: Quit application

#### Search Mode
- Type your search query
- `Enter`: Execute search
- `Esc`: Cancel and return to normal mode

#### Sort Mode (Advanced)
- Type sort commands like:
  - `timestamp asc` - Sort by timestamp ascending
  - `level desc` - Sort by log level descending  
  - `device` - Sort by device name (descending by default)
  - `temperature` - Sort by temperature
  - `humidity` - Sort by humidity
- `Enter`: Apply custom sort
- `Esc`: Cancel and return to normal mode

#### Limit Mode
- Enter a number to set how many logs to fetch (e.g., `500`, `1000`)
- `Enter`: Apply new limit and refresh
- `Esc`: Cancel and return to normal mode

#### Details Mode
- `Enter` or `Esc`: Close details and return to normal mode

## API Requirements

The TUI expects the log-forwarding API to be running and accessible. The API should provide:

- `GET /logs` - Query logs with optional parameters (limit, offset, level, device, from, to)
- `GET /logs/search` - Search logs with text query

## Building

```bash
cargo build --release
```