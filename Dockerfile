# syntax=docker/dockerfile:1

# Use latest stable Rust (supports Edition 2024)
FROM rust:1 AS builder
# (Optional) native deps commonly needed by crypto/net crates
RUN apt-get update && apt-get install -y pkg-config libssl-dev protobuf-compiler && rm -rf /var/lib/apt/lists/*
# Make sure rustup uses current stable inside the image
RUN rustup update stable && rustup default stable

WORKDIR /app

# Cache deps
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY node ./node

# Build by binary name
RUN cargo --version && rustc --version
RUN cargo build --release --locked --bin ippan-node

# ── Runtime ─────────────────────────────────────────────────────────────
FROM debian:12-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/ippan-node /usr/local/bin/ippan-node

EXPOSE 7070 8080
ENTRYPOINT ["/usr/local/bin/ippan-node"]
