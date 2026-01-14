# Build stage
FROM rust:1.91 AS builder

WORKDIR /usr/src/inflyte

# Copy manifests and source code together to avoid caching issues
COPY Cargo.toml ./
COPY src ./src

# Build the application in release mode
RUN cargo build --release

# Verify the binary was built with all dependencies
RUN ls -la target/release/inflyte && \
    ldd target/release/inflyte || true

# Runtime stage
FROM debian:bookworm-slim

# Install CA certificates and OpenSSL for HTTPS requests
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 file && \
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
ENV RUST_BACKTRACE=full
ENV RUST_LOG=debug
CMD ["sh", "-c", "echo 'Testing version:' && /usr/local/bin/inflyte --version 2>&1 && echo '---' && echo 'Testing with env var:' && /usr/local/bin/inflyte 2>&1; EXIT=$?; echo 'Exit code:' $EXIT; sleep 5"]
