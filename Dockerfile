FROM rust:1.96-slim-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked --bins

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install --no-install-recommends -y ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 app

WORKDIR /app
COPY --from=builder /build/target/release/server /app/server
COPY --from=builder /build/target/release/telegram_bot /app/telegram_bot

USER app
EXPOSE 8080

ENTRYPOINT ["/app/server"]
