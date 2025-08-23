RUN adduser -D appuser
USER appuser

FROM rust:trixie AS builder
WORKDIR /usr/src/log_sender
COPY . .
RUN cargo install --path .

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/log_sender /usr/local/bin/log_sender
CMD ["log_sender"]
