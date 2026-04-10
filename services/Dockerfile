# syntax=docker/dockerfile:1.7
#
# Shared multi-stage Dockerfile for all SolUPG services.
# Build a single service by setting the SERVICE build arg:
#
#   docker build --build-arg SERVICE=api-gateway      -t solupg/api-gateway:dev -f services/Dockerfile services
#   docker build --build-arg SERVICE=routing-engine   -t solupg/routing-engine:dev   -f services/Dockerfile services
#   docker build --build-arg SERVICE=directory-service -t solupg/directory-service:dev -f services/Dockerfile services
#   docker build --build-arg SERVICE=clearing-engine  -t solupg/clearing-engine:dev  -f services/Dockerfile services
#   docker build --build-arg SERVICE=monitoring       -t solupg/monitoring:dev       -f services/Dockerfile services
#
# Uses cargo-chef for layer-cached dependency compilation. The same crate
# graph is reused across all five service images, so the cook stage is
# cached aggressively in CI.

ARG RUST_VERSION=1.79.0
ARG DEBIAN_VERSION=bookworm

# ---------------------------------------------------------------------------
# Stage 1: chef base
# ---------------------------------------------------------------------------
FROM rust:${RUST_VERSION}-${DEBIAN_VERSION} AS chef
RUN cargo install cargo-chef --locked --version 0.1.67
WORKDIR /workspace

# ---------------------------------------------------------------------------
# Stage 2: planner — emit the dep recipe
# ---------------------------------------------------------------------------
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ---------------------------------------------------------------------------
# Stage 3: builder — cook dependencies then build the requested service
# ---------------------------------------------------------------------------
FROM chef AS builder
ARG SERVICE
ENV CARGO_TERM_COLOR=always
ENV RUSTFLAGS="-C target-cpu=x86-64-v2"

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        libpq-dev \
        ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY --from=planner /workspace/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN test -n "${SERVICE}" || (echo "ERROR: SERVICE build arg required" && exit 1)
RUN cargo build --release --locked -p "${SERVICE}"

# ---------------------------------------------------------------------------
# Stage 4: runtime — minimal debian with just the binary
# ---------------------------------------------------------------------------
FROM debian:${DEBIAN_VERSION}-slim AS runtime
ARG SERVICE
ENV SERVICE_NAME=${SERVICE}

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        libpq5 \
        curl \
 && rm -rf /var/lib/apt/lists/* \
 && groupadd --system --gid 10001 solupg \
 && useradd  --system --uid 10001 --gid solupg --home /app --shell /usr/sbin/nologin solupg \
 && mkdir -p /app \
 && chown -R solupg:solupg /app

WORKDIR /app
COPY --from=builder /workspace/target/release/${SERVICE} /usr/local/bin/solupg-service
COPY --from=builder /workspace/migrations /app/migrations

USER solupg:solupg

# Services honor a per-service *_PORT env var (see each main.rs).
# Default dev port mapping — override at compose/k8s level:
#   routing-engine    3000
#   directory-service 3001
#   api-gateway       3002
#   clearing-engine   3003
#   monitoring        3004
ENV RUST_LOG=info

HEALTHCHECK --interval=10s --timeout=3s --start-period=15s --retries=5 \
  CMD curl -fsS "http://127.0.0.1:${PORT:-3000}/health" || exit 1

ENTRYPOINT ["/usr/local/bin/solupg-service"]
