# Multi-stage build for HappyTest reader binary
# Optimized for small image size and security

# === Build stage ===
FROM rust:1.82-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create dummy src to build dependencies
RUN mkdir -p src/reader src/exchange/bybit src/storage/sink src/storage/source \
    src/core src/domain/indicator src/domain/model src/strategy src/backtest \
    src/analytics/pnl src/config src/utils

# Create minimal lib.rs for dependency building
RUN echo "pub mod core;" > src/lib.rs && \
    echo "pub mod reader { pub mod models; }" >> src/lib.rs && \
    echo "" > src/core/mod.rs && \
    echo "pub struct OrderbookData {}" > src/reader/models.rs && \
    echo "fn main() {}" > src/reader/main.rs && \
    echo "fn main() {}" > src/main.rs

# Build dependencies only (cached layer)
RUN cargo build --release --bin reader 2>/dev/null || true

# Copy actual source code
COPY src ./src

# Build the actual binary
RUN cargo build --release --bin reader

# === Runtime stage ===
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/reader /usr/local/bin/

# Create non-root user for security
RUN useradd -r -s /bin/false happytest

# Create data directory
RUN mkdir -p /data && chown happytest:happytest /data

# Switch to non-root user
USER happytest

# Set working directory
WORKDIR /data

# Default entrypoint
ENTRYPOINT ["reader"]
CMD ["--help"]
