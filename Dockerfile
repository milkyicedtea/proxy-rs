# Deps planner
FROM rust:latest AS planner
WORKDIR /app

RUN cargo install cargo-chef
COPY Cargo.toml Cargo.lock ./

RUN cargo chef prepare --recipe-path recipe.json


# Deps build cache
FROM rust:latest AS cacher
WORKDIR /app

RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json


# Build stage
FROM rust:latest AS builder
WORKDIR /app

COPY . .
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo

# Optional if running and building on same host
# ENV RUSTFLAGS="-C target-cpu=native"

RUN cargo build --release

# Runtime
FROM debian:bookworm-slim
WORKDIR /app

# copy binary
COPY --from=builder /app/target/release/proxy-rs /usr/local/bin/proxy-rs

EXPOSE 25565

CMD ["proxy-rs"]
