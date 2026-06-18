# syntax=docker/dockerfile:1

# ---- frontend build (our frontend/ SPA on @excalidraw/excalidraw, built with bun) ----
FROM --platform=$BUILDPLATFORM oven/bun:1 AS frontend
WORKDIR /app/frontend
# Install deps against the lockfile first so this layer caches across source-only changes.
COPY frontend/package.json frontend/bun.lock ./
RUN bun install --frozen-lockfile
COPY frontend/ ./
# vite's outDir is ../backend/crates/server/assets, so the build writes to
# /app/backend/crates/server/assets. Create the parent so vite emits into a known path even
# though backend/ isn't in this stage.
RUN mkdir -p /app/backend/crates/server && bun run build

# ---- backend dependency cache (cargo-chef) ----
FROM rust:1-slim AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
WORKDIR /app/backend
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/backend/recipe.json backend/recipe.json
WORKDIR /app/backend
RUN cargo chef cook --release --recipe-path recipe.json
WORKDIR /app
COPY . .
# Embed the built frontend (fills the .gitkeep-only assets dir) before compiling.
COPY --from=frontend /app/backend/crates/server/assets/ backend/crates/server/assets/
WORKDIR /app/backend
RUN cargo build --release --bin oxydraw && cp target/release/oxydraw /oxydraw

# ---- runtime ----
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /oxydraw /usr/local/bin/oxydraw
# Run as a non-root system user; /data is its writable home for the default
# SQLite file (DATA_SOURCE_NAME defaults to oxydraw.db, relative to the workdir).
RUN useradd --system --create-home --home-dir /data --shell /usr/sbin/nologin oxydraw
WORKDIR /data
USER oxydraw
EXPOSE 3002
ENV LISTEN=0.0.0.0:3002
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -fsS http://localhost:3002/ || exit 1
ENTRYPOINT ["/usr/local/bin/oxydraw"]
