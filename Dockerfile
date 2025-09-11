# IPPAN Blockchain Node - Multi-stage Docker build
FROM rust:1.81-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    curl \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Install Rust nightly toolchain for edition2024 dependencies
RUN rustup toolchain install nightly && rustup default nightly

# Set working directory
WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY apps/neuro-cli/ apps/neuro-cli/
COPY benches/ benches/

# Create dummy main.rs for dependency compilation
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies with nightly toolchain
RUN cargo +nightly build --release --bin ippan
RUN rm src/main.rs

# Copy source code
COPY src/ src/
COPY config/ config/
COPY scripts/ scripts/

# Build the application with nightly toolchain
RUN cargo +nightly build --release --bin ippan

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    jq \
    openssl \
    && rm -rf /var/lib/apt/lists/*

# Create ippan user
RUN groupadd -r ippan && useradd -r -g ippan ippan

# Create required directories and a writable home for the ippan user
RUN mkdir -p /data /keys /logs /config /home/ippan && \
    chown -R ippan:ippan /data /keys /logs /config /home/ippan

# Copy the built binary
COPY --from=builder /app/target/release/ippan /usr/local/bin/ippan

# Copy configuration files
COPY --from=builder /app/config/ /config/
COPY --from=builder /app/scripts/ /scripts/
COPY entrypoint.sh /entrypoint.sh

# Make scripts executable
RUN chmod +x /scripts/*.sh /entrypoint.sh

# Switch to ippan user
USER ippan

# Expose ports
EXPOSE 8080 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:3000/api/v1/status || exit 1

# Set environment variables
ENV HOME=/home/ippan \
    XDG_CONFIG_HOME=/config \
    RUST_LOG=info \
    IPPAN_CONFIG_PATH=/config/ippan-config.json \
    IPPAN_DATA_DIR=/data \
    IPPAN_KEYS_DIR=/keys \
    IPPAN_LOG_DIR=/logs

# Start the application
ENTRYPOINT ["/entrypoint.sh"]
