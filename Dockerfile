# ---- Build stage ------------------------------------------------------------
FROM rust:1.88-slim AS builder

# Native deps commonly needed by Rust crates (openssl, protobuf, etc.)
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates clang llvm protobuf-compiler \
  && rm -rf /var/lib/apt/lists/*

# Enable incremental builds and useful diagnostics
ENV CARGO_TERM_COLOR=always \
    RUST_BACKTRACE=1

WORKDIR /app

# Pre-cache dependencies:
#  - Copy manifests first so `cargo fetch` can be cached
COPY Cargo.toml Cargo.lock ./
# If you use a workspace, copy each member's Cargo.toml
# (adjust the list below to your actual workspace layout)
COPY crates/consensus/Cargo.toml crates/consensus/Cargo.toml
COPY crates/crypto/Cargo.toml    crates/crypto/Cargo.toml
COPY crates/mempool/Cargo.toml   crates/mempool/Cargo.toml
COPY crates/p2p/Cargo.toml       crates/p2p/Cargo.toml
COPY crates/rpc/Cargo.toml       crates/rpc/Cargo.toml
COPY crates/storage/Cargo.toml   crates/storage/Cargo.toml
COPY crates/types/Cargo.toml     crates/types/Cargo.toml
COPY crates/core/Cargo.toml      crates/core/Cargo.toml
COPY crates/network/Cargo.toml   crates/network/Cargo.toml
COPY crates/time/Cargo.toml      crates/time/Cargo.toml
COPY node/Cargo.toml             node/Cargo.toml

# Create empty src to satisfy cargo before copying full sources
RUN mkdir -p \
        crates/consensus/src \
        crates/crypto/src \
        crates/mempool/src \
        crates/p2p/src \
        crates/rpc/src \
        crates/storage/src \
        crates/types/src \
        node/src \
    && echo "pub fn placeholder() {}" > crates/consensus/src/lib.rs \
    && echo "pub fn placeholder() {}" > crates/crypto/src/lib.rs \
    && echo "pub fn placeholder() {}" > crates/mempool/src/lib.rs \
    && echo "pub fn placeholder() {}" > crates/p2p/src/lib.rs \
    && echo "pub fn placeholder() {}" > crates/rpc/src/lib.rs \
    && echo "pub fn placeholder() {}" > crates/storage/src/lib.rs \
    && echo "pub fn placeholder() {}" > crates/types/src/lib.rs \
    && mkdir -p crates/core/src crates/network/src crates/time/src \
    && echo "pub fn placeholder() {}" > crates/core/src/lib.rs \
    && echo "pub fn placeholder() {}" > crates/network/src/lib.rs \
    && echo "pub fn placeholder() {}" > crates/time/src/lib.rs \
    && echo "fn main() {}" > node/src/main.rs

# Fetch and build dependencies only (this layer will cache well)
RUN cargo build --release --workspace --locked || true

# Now copy the full sources
COPY . .

# Build with full verbosity so CI logs show the first real error
# Use --locked to respect Cargo.lock in CI
RUN cargo build --release --workspace --locked --verbose


# ---- Runtime stage ---------------------------------------------------------
FROM debian:bullseye-slim AS runtime
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    ca-certificates \
  && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 10001 ippan

WORKDIR /app
# Copy the binaries you actually ship (adjust names)
COPY --from=builder /app/target/release/ippan-node /usr/local/bin/ippan-node


RUN chown -R ippan:ippan /app

# Drop privileges
USER ippan

# Healthcheck if you expose HTTP: adjust as needed
# HEALTHCHECK --interval=30s --timeout=3s CMD curl -fsS http://localhost:3000/health || exit 1

CMD ["ippan-node"]
