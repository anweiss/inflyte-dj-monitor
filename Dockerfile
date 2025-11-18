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

# Install CA certificates for HTTPS requests
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /usr/src/inflyte/target/release/inflyte /usr/local/bin/inflyte

# Run as non-root user
RUN useradd -m -u 1000 inflyte
USER inflyte

WORKDIR /home/inflyte

# Run the application
# Option 1: Use INFLYTE_URLS environment variable (comma-separated URLs)
# Option 2: Mount a urls.txt file at /home/inflyte/urls.txt
CMD ["sh", "-c", "echo 'Starting Inflyte DJ Monitor...'; \
    if [ -f urls.txt ]; then \
        echo 'Using urls.txt file'; \
        exec inflyte --file urls.txt; \
    elif [ -n \"${INFLYTE_URLS}\" ]; then \
        echo \"Using INFLYTE_URLS: ${INFLYTE_URLS}\"; \
        exec inflyte --url \"${INFLYTE_URLS}\"; \
    else \
        echo 'Error: Either mount urls.txt or set INFLYTE_URLS environment variable' >&2; \
        exit 1; \
    fi"]
