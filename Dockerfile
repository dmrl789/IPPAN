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
# Copy all workspace member Cargo.toml files
COPY crates/ai_core/Cargo.toml           crates/ai_core/Cargo.toml
COPY crates/ai_registry/Cargo.toml       crates/ai_registry/Cargo.toml
COPY crates/ai_service/Cargo.toml        crates/ai_service/Cargo.toml
COPY crates/benchmark/Cargo.toml         crates/benchmark/Cargo.toml
COPY crates/cli/Cargo.toml               crates/cli/Cargo.toml
COPY crates/consensus/Cargo.toml         crates/consensus/Cargo.toml
COPY crates/consensus_dlc/Cargo.toml     crates/consensus_dlc/Cargo.toml
COPY crates/core/Cargo.toml              crates/core/Cargo.toml
COPY crates/crypto/Cargo.toml            crates/crypto/Cargo.toml
COPY crates/economics/Cargo.toml         crates/economics/Cargo.toml
COPY crates/explorer/Cargo.toml          crates/explorer/Cargo.toml
COPY crates/governance/Cargo.toml        crates/governance/Cargo.toml
COPY crates/ippan_economics/Cargo.toml   crates/ippan_economics/Cargo.toml
COPY crates/keygen/Cargo.toml            crates/keygen/Cargo.toml
COPY crates/l1_handle_anchors/Cargo.toml crates/l1_handle_anchors/Cargo.toml
COPY crates/l2_fees/Cargo.toml           crates/l2_fees/Cargo.toml
COPY crates/l2_handle_registry/Cargo.toml crates/l2_handle_registry/Cargo.toml
COPY crates/mempool/Cargo.toml           crates/mempool/Cargo.toml
COPY crates/network/Cargo.toml           crates/network/Cargo.toml
COPY crates/p2p/Cargo.toml               crates/p2p/Cargo.toml
COPY crates/rpc/Cargo.toml               crates/rpc/Cargo.toml
COPY crates/security/Cargo.toml          crates/security/Cargo.toml
COPY crates/storage/Cargo.toml           crates/storage/Cargo.toml
COPY crates/time/Cargo.toml              crates/time/Cargo.toml
COPY crates/treasury/Cargo.toml          crates/treasury/Cargo.toml
COPY crates/types/Cargo.toml             crates/types/Cargo.toml
COPY crates/validator_resolution/Cargo.toml crates/validator_resolution/Cargo.toml
COPY crates/wallet/Cargo.toml            crates/wallet/Cargo.toml
COPY node/Cargo.toml                     node/Cargo.toml

# Create empty src to satisfy cargo before copying full sources
RUN for crate in ai_core ai_registry ai_service benchmark cli consensus consensus_dlc \
                 core crypto economics explorer governance ippan_economics keygen \
                 l1_handle_anchors l2_fees l2_handle_registry mempool network p2p \
                 rpc security storage time treasury types validator_resolution wallet; do \
      mkdir -p crates/$crate/src && echo "pub fn placeholder() {}" > crates/$crate/src/lib.rs; \
    done \
    && mkdir -p node/src \
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
