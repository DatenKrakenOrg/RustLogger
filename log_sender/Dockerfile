ARG RUST_VERSION=1.89.0
ARG APP_NAME=log_sender

FROM rust:${RUST_VERSION}-alpine AS build
ARG APP_NAME
WORKDIR /app

# Install build dependencies
RUN apk add --no-cache musl-dev gcc pkgconfig openssl-dev openssl-libs-static clang lld git

# Build with cache mounts
RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    cargo build --locked --release && \
    cp ./target/release/log_sender /usr/local/bin/log_sender

FROM alpine:3.22

RUN apk add --no-cache curl

RUN adduser -D appuser
USER appuser

COPY --from=build /usr/local/bin/log_sender /usr/local/bin/log_sender
CMD ["log_sender"]
