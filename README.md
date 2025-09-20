# RustLogger
## Quick Start
Navigate into project folder
### Start RustLogger with Docker
```bash
cd docker-compose
docker compose up -d --build
```
### Launch Log TUI
```bash
cd log-tui
cargo build --release
cargo run
```

## Setup Development Environment
### Install Pre Commits
```bash
pipx install pre-commit 
```
**Navigate into project folder**

```bash
pre-commit install
pre-commit run
```

### Install Rust
<a href="https://www.rust-lang.org/learn/get-started">Take a look here</a>

