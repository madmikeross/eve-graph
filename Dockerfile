FROM messense/rust-musl-cross:x86_64-musl as chef
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

# Create a new stage with minimal image
FROM scratch
COPY --from=builder /eve-graph/target/x86_64-unknown-linux-musl/release/eve-graph /eve-graph
ENTRYPOINT ["/eve-graph"]
EXPOSE 8008