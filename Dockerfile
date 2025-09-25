# syntax=docker/dockerfile:1

FROM rust:1.80 AS builder
# If any crate needs native deps (safe to include; tiny impact)
RUN apt-get update && apt-get install -y pkg-config libssl-dev protobuf-compiler && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY node ./node

# IMPORTANT: build by *binary* name, not package
RUN cargo build --release --locked --bin ippan-node

# ── Runtime ─────────────────────────────────────────────────────────────
FROM debian:12-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/ippan-node /usr/local/bin/ippan-node

EXPOSE 7070 8080
ENTRYPOINT ["/usr/local/bin/ippan-node"]
