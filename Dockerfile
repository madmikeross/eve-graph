# Use the official Rust image, which will respect the rust-toolchain.toml file
FROM rust:1.88.0 as chef
# Install musl-tools for static compilation and cargo-chef for caching
RUN apt-get update && apt-get install -y musl-tools && rm -rf /var/lib/apt/lists/*
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo install cargo-chef
WORKDIR /eve-graph

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /eve-graph/recipe.json recipe.json
# Build & cache deps
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
# Copy the source
COPY . .
# Build the app
RUN cargo build --release --target x86_64-unknown-linux-musl

# Create a new, minimal stage for the final image
FROM alpine:latest
# Add SSL certificates for HTTPS requests
RUN apk --no-cache add ca-certificates
COPY --from=builder /eve-graph/target/x86_64-unknown-linux-musl/release/eve-graph /eve-graph
ENTRYPOINT ["/eve-graph"]
EXPOSE 8008