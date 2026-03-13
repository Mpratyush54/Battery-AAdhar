FROM rust:latest AS builder

# Install protoc since tonic-build requires it
RUN apt-get update && apt-get install -y protobuf-compiler pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Copy the protobuf
COPY proto ./proto

# Copy the core code
COPY core ./core

# Build the Rust server
WORKDIR /usr/src/app/core
RUN cargo build --release

FROM debian:bullseye-slim

# Install OpenSSL required by sqlx
RUN apt-get update && apt-get install -y libssl1.1 ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/app/core/target/release/core .

# Make sure it can connect to the DB container correctly.
# We'll pass DATABASE_URL via docker-compose environment variable,
# but we can copy the .env file as well if it exists.
COPY core/.env .env

EXPOSE 50051

CMD ["./core"]
