ARG RUST_VERSION=latest
ARG APP_NAME=log-forwarding-api

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
FROM bitnami/minideb:bookworm

# add dependencies for run-container
RUN apt-get update && apt-get install -y libssl-dev curl

WORKDIR /app

USER 1001

EXPOSE 8080

COPY --from=builder /code/target/release/log-forwarding-api log-forwarding-api
CMD ["/app/log-forwarding-api"]
