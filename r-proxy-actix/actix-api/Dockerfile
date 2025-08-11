FROM rust:trixie AS builder
WORKDIR /usr/src/actix-api
COPY . .
RUN cargo install --path .

FROM debian:trixie-slim
# RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/actix-api /usr/local/bin/actix-api
CMD ["actix-api"]
