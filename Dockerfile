# Build stage
FROM rust:1.82 as builder

WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm src/main.rs

# Copy source code and static files
COPY src/ src/
COPY static/ static/

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install SSL certificates
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create cache directory for ACME certificates
RUN mkdir -p /var/cache/acme

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/matthewblair-net /app/matthewblair-net

# Create non-root user
RUN useradd -r -s /bin/false appuser
RUN chown -R appuser:appuser /app /var/cache/acme
USER appuser

EXPOSE 80 443

ENTRYPOINT ["/app/matthewblair-net"]
