# Build stage
FROM rust:1.91 AS builder

WORKDIR /usr/src/inflyte

# Copy manifests
COPY Cargo.toml ./

# Create a dummy main.rs to build dependencies first
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src target/release/inflyte*

# Copy actual source code
COPY src ./src

# Build the application in release mode
# This will reuse cached dependencies from the previous layer
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install CA certificates and OpenSSL for HTTPS requests
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /usr/src/inflyte/target/release/inflyte /usr/local/bin/inflyte

# Run as non-root user
RUN useradd -m -u 1000 inflyte
USER inflyte

WORKDIR /home/inflyte

# Run the application
# URLs can be provided via:
# 1. INFLYTE_URLS environment variable (comma-separated)
# 2. Mount a urls.txt file at /home/inflyte/urls.txt and use --file flag
# 3. Pass --url flags directly
CMD ["sh", "-c", "echo 'Starting inflyte...' && echo 'Checking binary:' && ls -la /usr/local/bin/inflyte && echo 'Library dependencies:' && ldd /usr/local/bin/inflyte 2>&1 || true && echo 'Running inflyte...' && if [ -f urls.txt ]; then exec inflyte --file urls.txt; else exec inflyte; fi"]
