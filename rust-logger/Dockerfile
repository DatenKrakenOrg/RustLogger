ARG RUST_VERSION=latest
ARG APP_NAME=rust-logger

FROM rust:${RUST_VERSION} AS builder
ARG APP_NAME
WORKDIR /code

# Install host build dependencies.
RUN apt-get update && apt-get install -y libssl-dev

# fetch all dependencies and cache it for further builds
RUN cargo init

# Layer Build of defined dependencies
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
RUN cargo fetch
# Prebuild libs with cache
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release

# copy actual source code
COPY src/ src/

# Test and build with cache from above (contains workaround)
RUN --mount=type=cache,target=/usr/local/cargo/registry <<EOF
  set -e
  # update timestamps to force a new build
  touch /code/src/main.rs
  #cargo test --release
  cargo build --release
EOF

# Actual container
FROM ubuntu:24.04

# add dependencies for run-container
RUN apt-get update && apt-get install -y libssl-dev 

WORKDIR /app

COPY --from=builder /code/target/release/${APP_NAME} ${APP_NAME}

USER 1001

# What the container should run when it is started.
CMD ["/app/rust-logger", "--config-path", "/etc/config/message_types.toml", "--path", "/etc/logs/output.csv", "--count", "1000"]
