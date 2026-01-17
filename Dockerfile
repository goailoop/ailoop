# Multi-stage Dockerfile for ailoop-cli
# Build stage - using smaller base image
FROM rust:1.75-alpine AS builder

# Install required packages for building
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    pkgconfig

# Set the working directory
WORKDIR /app

# Copy the workspace files
COPY Cargo.toml Cargo.lock ./
COPY ailoop-core/ ./ailoop-core/
COPY ailoop-cli/ ./ailoop-cli/

# Build the ailoop-cli binary in release mode with optimizations
RUN cargo build --release --bin ailoop

# Strip the binary to reduce size
RUN strip /app/target/release/ailoop

# Runtime stage - using distroless for minimal size
FROM gcr.io/distroless/static-debian12:nonroot

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/ailoop /usr/local/bin/ailoop

# Expose ports for WebSocket (8080) and HTTP API (8081)
EXPOSE 8080 8081

# Set the default command
CMD ["/usr/local/bin/ailoop", "serve"]
