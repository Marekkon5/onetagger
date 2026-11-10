# syntax=docker/dockerfile:1.6

# ---- Build stage ----------------------------------------------------------
FROM rust:1-bookworm AS builder

ARG DEBIAN_FRONTEND=noninteractive

# Build deps:
# - libasound2-dev: onetagger-player links alsa even when the binary runs in server mode
# - lld: matches .cargo/config.toml's `link-arg=-fuse-ld=lld` for x86_64-unknown-linux-gnu
# - nodejs/npm + pnpm: build the Vue/Quasar client (client/dist) which onetagger-ui embeds via include_dir!
# We skip libwebkit2gtk-4.1-dev: only the `onetagger` GUI crate needs it, and we build onetagger-cli only.
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        libssl-dev \
        libasound2-dev \
        ca-certificates \
        git \
        lld \
        nodejs \
        npm \
    && rm -rf /var/lib/apt/lists/* \
    && npm install -g pnpm@8

WORKDIR /src

# Source comes from this repo (Dockerfile lives at the repo root, alongside Cargo.toml, crates/, client/).
# .dockerignore filters target/, node_modules/, dist/, .git/ so local edits are picked up without bloating context.
COPY . .

# Client must be built before cargo: onetagger-ui's include_dir!("../../client/dist") runs at compile time.
# client/.gitignore excludes pnpm-lock.yaml, so checkouts never have one — `pnpm install` (not `--frozen-lockfile`).
RUN cd client && pnpm install && pnpm run build

# -p onetagger-cli skips the GUI binary (webkit/gtk transitive deps).
RUN cargo build --release -p onetagger-cli


# ---- Runtime stage --------------------------------------------------------
FROM debian:bookworm-slim AS runtime

ARG DEBIAN_FRONTEND=noninteractive

# Runtime libs:
# - libasound2: dynamic alsa link from onetagger-player
# - libssl3: openssl runtime for HTTPS calls to MusicBrainz/Discogs/Spotify/Beatport/...
# - ca-certificates: trust store for those outbound calls
RUN apt-get update && apt-get install -y --no-install-recommends \
        libasound2 \
        libssl3 \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1000 --user-group --shell /usr/sbin/nologin onetagger

COPY --from=builder /src/target/release/onetagger-cli /usr/local/bin/onetagger-cli

USER onetagger
WORKDIR /home/onetagger
ENV HOME=/home/onetagger

# Server binds 0.0.0.0:36913 with --expose; without it the bind is 127.0.0.1 which is
# unreachable from outside the container.
EXPOSE 36913

ENTRYPOINT ["onetagger-cli"]
CMD ["server", "--expose"]
