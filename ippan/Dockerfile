# Multi-stage Dockerfile for IPPAN Blockchain
FROM rust:1.75-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY rust-toolchain.toml ./
COPY .cargo/ .cargo/

# Copy crate manifests
COPY crates/common/Cargo.toml crates/common/
COPY crates/node/Cargo.toml crates/node/
COPY crates/wallet-cli/Cargo.toml crates/wallet-cli/
COPY crates/loadgen-cli/Cargo.toml crates/loadgen-cli/
COPY crates/bench/Cargo.toml crates/bench/

# Create dummy source files for initial build
RUN mkdir -p crates/common/src crates/node/src crates/wallet-cli/src crates/loadgen-cli/src crates/bench/src
RUN echo "fn main() {}" > crates/common/src/lib.rs
RUN echo "fn main() {}" > crates/node/src/main.rs
RUN echo "fn main() {}" > crates/wallet-cli/src/main.rs
RUN echo "fn main() {}" > crates/loadgen-cli/src/main.rs
RUN echo "fn main() {}" > crates/bench/src/lib.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release

# Copy actual source code
COPY crates/common/src/ crates/common/src/
COPY crates/node/src/ crates/node/src/
COPY crates/wallet-cli/src/ crates/wallet-cli/src/
COPY crates/loadgen-cli/src/ crates/loadgen-cli/src/
COPY crates/bench/src/ crates/bench/src/
COPY crates/bench/benches/ crates/bench/benches/

# Build the applications
RUN cargo build --release

# Node stage
FROM debian:bookworm-slim as node

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false app

# Set working directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/ippan-node /app/ippan-node

# Create necessary directories
RUN mkdir -p /app/data /app/logs && chown -R app:app /app

# Switch to app user
USER app

# Expose ports
EXPOSE 8080 8081

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Default command
CMD ["/app/ippan-node", "--http-port", "8080", "--p2p-port", "8081", "--shards", "4"]

# Load generator stage
FROM debian:bookworm-slim as loadgen

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false app

# Set working directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/ippan-loadgen-cli /app/ippan-loadgen-cli

# Create results directory
RUN mkdir -p /app/results && chown -R app:app /app

# Switch to app user
USER app

# Default command
CMD ["/app/ippan-loadgen-cli", "--tps", "1000", "--accounts", "100", "--duration", "60", "--nodes", "http://localhost:8080"]

# Wallet CLI stage
FROM debian:bookworm-slim as wallet

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false app

# Set working directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/ippan-wallet-cli /app/ippan-wallet-cli

# Create wallet directory
RUN mkdir -p /app/wallets && chown -R app:app /app

# Switch to app user
USER app

# Default command
CMD ["/app/ippan-wallet-cli", "--help"]

# Development stage
FROM rust:1.75-slim as dev

# Install development dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    git \
    vim \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy source code
COPY . .

# Create necessary directories
RUN mkdir -p /app/data /app/logs /app/results

# Expose ports
EXPOSE 8080 8081

# Default command
CMD ["bash"]
