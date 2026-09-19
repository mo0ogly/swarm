# syntax=docker/dockerfile:1
FROM golang:1.26-bookworm AS build
WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 go build -trimpath -o /out/swarm .

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
