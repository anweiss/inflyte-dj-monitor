# Build stage
FROM rust:1.91 AS builder

WORKDIR /usr/src/inflyte

# Copy manifests
COPY Cargo.toml ./

# Copy source code
COPY src ./src

# Build the application in release mode
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install CA certificates for HTTPS requests
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /usr/src/inflyte/target/release/inflyte /usr/local/bin/inflyte

# Run as non-root user
RUN useradd -m -u 1000 inflyte
USER inflyte

# Run the application
# INFLYTE_URLS environment variable should be provided at runtime (comma-separated URLs)
CMD ["sh", "-c", "inflyte --url ${INFLYTE_URLS}"]
