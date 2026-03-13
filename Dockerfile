FROM rust:1-bookworm AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev clang cmake \
    && rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build --release -p sss-api -p sss-indexer

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/sss-api /usr/local/bin/sss-api
COPY --from=builder /app/target/release/sss-indexer /usr/local/bin/sss-indexer

ENV RUST_LOG=info

