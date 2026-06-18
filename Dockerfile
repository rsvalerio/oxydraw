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
RUN cd backend && cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/backend/recipe.json backend/recipe.json
RUN cd backend && cargo chef cook --release --recipe-path recipe.json
COPY . .
# Embed the built frontend (fills the .gitkeep-only assets dir) before compiling.
COPY --from=frontend /app/backend/crates/server/assets/ backend/crates/server/assets/
RUN cd backend && cargo build --release --bin oxydraw && cp target/release/oxydraw /oxydraw

# ---- runtime ----
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /oxydraw /usr/local/bin/oxydraw
EXPOSE 3002
ENV LISTEN=0.0.0.0:3002
ENTRYPOINT ["/usr/local/bin/oxydraw"]
