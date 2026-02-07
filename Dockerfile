FROM rust:1.76-slim-bookworm as builder

WORKDIR /usr/src/app
COPY . .

# Install dependencies if needed (e.g. ssl)
RUN apt-get update && apt-get install -y pkg-config libssl-dev

# Build the application
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/paperforge-rs /usr/local/bin/paperforge-rs

# Expose port
EXPOSE 3000

# Set environment
ENV RUST_LOG=info

CMD ["paperforge-rs"]
