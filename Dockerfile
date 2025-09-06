# IPPAN Blockchain Node - Multi-stage Docker build
FROM rust:1.75 as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    curl \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY apps/neuro-cli/ apps/neuro-cli/

# Create dummy main.rs for dependency compilation
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies
RUN cargo build --release --bin ippan
RUN rm src/main.rs

# Copy source code
COPY src/ src/
COPY config/ config/
COPY scripts/ scripts/

# Build the application
RUN cargo build --release --bin ippan

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

# Create required directories
RUN mkdir -p /data /keys /logs /config && \
    chown -R ippan:ippan /data /keys /logs /config

# Copy the built binary
COPY --from=builder /app/target/release/ippan /usr/local/bin/ippan

# Copy configuration files
COPY --from=builder /app/config/ /config/
COPY --from=builder /app/scripts/ /scripts/

# Make scripts executable
RUN chmod +x /scripts/*.sh

# Switch to ippan user
USER ippan

# Expose ports
EXPOSE 8080 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:3000/api/v1/status || exit 1

# Set environment variables
ENV RUST_LOG=info
ENV IPPAN_CONFIG_PATH=/config/default.toml
ENV IPPAN_DATA_DIR=/data
ENV IPPAN_KEYS_DIR=/keys
ENV IPPAN_LOG_DIR=/logs

# Start the application
CMD ["ippan"]
