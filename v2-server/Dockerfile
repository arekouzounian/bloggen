# Build stage
FROM rust:1.83-bookworm AS builder

WORKDIR /build

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency manifests
# Build context is parent directory, so paths are relative to bloggen/
COPY v2-server/Cargo.toml v2-server/Cargo.lock ./
COPY client/bgc-ast/Cargo.toml ./client/bgc-ast/

# Create dummy source files to cache dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    mkdir -p client/bgc-ast/src && \
    echo "" > client/bgc-ast/src/lib.rs

# Build dependencies (cached layer)
RUN cargo build --release && \
    rm -rf src client/bgc-ast/src

# Copy actual source code
COPY v2-server/src ./src
COPY client/bgc-ast ./client/bgc-ast

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 -s /bin/bash bloggen

# Create necessary directories
RUN mkdir -p /app/migrations /etc/bloggen /var/log/bloggen && \
    chown -R bloggen:bloggen /app /etc/bloggen /var/log/bloggen

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/v2-server /app/bloggen-server

# Copy migrations (relative to build context = bloggen/)
COPY v2-server/migrations /app/migrations

# Copy default config (can be overridden)
COPY v2-server/config.example.json /etc/bloggen/config.json

USER bloggen

EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

ENTRYPOINT ["/app/bloggen-server"]
