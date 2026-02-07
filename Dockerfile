FROM rust:1.78-slim-bookworm as builder

WORKDIR /usr/src/app

# Install build dependencies (including protobuf-compiler for gRPC)
RUN apt-get update && apt-get install -y pkg-config libssl-dev protobuf-compiler

# Copy entire workspace
COPY . .

# Build the gateway binary
RUN cargo build --release --bin gateway

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the gateway binary
COPY --from=builder /usr/src/app/target/release/gateway /usr/local/bin/gateway

# Expose port
EXPOSE 3000

# Set environment
ENV RUST_LOG=info

CMD ["gateway"]
