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
## API Requirements

The TUI expects the log-forwarding API to be running and accessible. The API should provide:

- `GET /logs` - Query logs with optional parameters (limit, offset, level, device, from, to)
- `GET /logs/search` - Search logs with text query

## Building

```bash
cargo build --release
```