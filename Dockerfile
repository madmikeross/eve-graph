FROM messense/rust-musl-cross:x86_64-musl as builder
WORKDIR /eve-graph
# Copy the source
COPY . .
# Build the app
RUN cargo build --release --target x86_64-unknown-linux-musl

# Create a new stage with minimal image
FROM scratch
COPY --from=builder /eve-graph/target/x86_64-unknown-linux-musl/release/eve-graph /eve-graph
ENTRYPOINT ["/eve-graph"]
EXPOSE 8008