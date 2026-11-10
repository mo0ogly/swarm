# syntax=docker/dockerfile:1
FROM rust:1.96-bookworm AS build
WORKDIR /src
COPY . .
# Default product features only; no fixtures or Go fallback. SQLite uses the
# checked-in .cargo/config.toml FILESTAT capability. Cargo.lock pins dependencies.
# A source archive has no Git attestation: absent metadata stays devel/unknown.
ARG SWARM_NATIVE_BUILD_METADATA
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/rust/target \
    if [ -n "$SWARM_NATIVE_BUILD_METADATA" ]; then export SWARM_NATIVE_BUILD_METADATA; else unset SWARM_NATIVE_BUILD_METADATA; fi; \
    cargo build --locked --manifest-path rust/Cargo.toml --release -p swarm-cli --bin swarm-rust \
    && install -D -m 755 rust/target/release/swarm-rust /out/swarm \
    && /out/swarm version --json

FROM node:22-bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates git python3 curl procps \
    && rm -rf /var/lib/apt/lists/* \
    && groupmod --new-name swarm node \
    && usermod --login swarm --home /home/swarm --move-home node
ENV HOME=/home/swarm \
    NPM_CONFIG_PREFIX=/home/swarm/.local \
    PATH=/home/swarm/.local/bin:$PATH \
    SWARM_ROOT=/workspace \
    SWARM_PORT=18787
COPY --from=build /out/swarm /usr/local/bin/swarm
COPY deploy/entrypoint.sh /usr/local/bin/swarm-entrypoint
COPY LICENSE THIRD_PARTY_NOTICES.md /usr/share/doc/swarm/
RUN chmod 755 /usr/local/bin/swarm-entrypoint
WORKDIR /workspace
USER 1000:1000
ENTRYPOINT ["/usr/local/bin/swarm-entrypoint"]
CMD ["web"]
