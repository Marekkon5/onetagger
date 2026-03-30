# syntax=docker/dockerfile:1.7

FROM node:20-bookworm-slim AS client-builder

WORKDIR /app/client

RUN --mount=type=secret,id=npmrc,target=/root/.npmrc,required=false \
    npm install -g pnpm@8

COPY client/package.json ./
RUN --mount=type=secret,id=npmrc,target=/root/.npmrc,required=false \
    --mount=type=cache,target=/root/.local/share/pnpm/store \
    pnpm install --no-frozen-lockfile

COPY client/ ./
RUN pnpm run build


FROM rust:1.86-bookworm AS builder

ARG GITHUB_SHA=unknown
ENV GITHUB_SHA=${GITHUB_SHA}

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        g++ \
        git \
        libasound2-dev \
        lld \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY assets ./assets
COPY crates ./crates
COPY --from=client-builder /app/client/dist ./client/dist

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/app/target \
    cargo build --locked --release -p onetagger-cli \
    && cp /app/target/release/onetagger-cli /app/onetagger-cli


FROM debian:bookworm-slim AS runtime

LABEL org.opencontainers.image.source="https://github.com/Marekkon5/onetagger"

ENV HOME=/tmp/onetagger-home \
    XDG_CONFIG_HOME=/data

WORKDIR /music

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        gosu \
        libasound2 \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /data /music /tmp/onetagger-home \
    && chmod 1777 /tmp

COPY --from=builder /app/onetagger-cli /usr/local/bin/onetagger-cli
COPY --chmod=755 docker/entrypoint.sh /usr/local/bin/docker-entrypoint.sh

ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["--help"]
