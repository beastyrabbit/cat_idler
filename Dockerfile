# syntax=docker/dockerfile:1.7

ARG RUST_VERSION=1.96
FROM rust:${RUST_VERSION}-bookworm AS builder

ARG TRUNK_VERSION=0.21.14
RUN apt-get update \
    && apt-get install --yes --no-install-recommends binaryen ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add wasm32-unknown-unknown
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo install --locked --version "${TRUNK_VERSION}" trunk

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY public ./public

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --locked --release -p cat-server
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cd crates/cat-web \
    && trunk build --release --locked

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 cat-server \
    && useradd --system --uid 10001 --gid cat-server --home-dir /app cat-server \
    && install --directory --owner=cat-server --group=cat-server /app /data

COPY --from=builder --chown=cat-server:cat-server /src/target/release/cat-server /app/cat-server
COPY --from=builder --chown=cat-server:cat-server /src/crates/cat-web/dist /app/web
COPY --from=builder --chown=cat-server:cat-server /src/public/images /app/public-images

ENV BIND_ADDR=0.0.0.0 \
    PORT=8787 \
    NODE_ENV=production \
    GAME_DB_PATH=/data/cat.db \
    CAT_SERVER_WEB_DIST_DIR=/app/web \
    CAT_SERVER_PUBLIC_IMAGES_DIR=/app/public-images

VOLUME ["/data"]
EXPOSE 8787
USER cat-server

HEALTHCHECK --interval=15s --timeout=3s --start-period=10s --retries=3 \
    CMD curl --fail --silent --show-error "http://127.0.0.1:${PORT}/ready" >/dev/null || exit 1

ENTRYPOINT ["/app/cat-server"]
