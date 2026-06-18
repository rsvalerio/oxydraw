---
id: TASK-0072
title: >-
  CONC-4: Graceful shutdown only listens for Ctrl-C — the packaged systemd
  deployment stops with SIGTERM and never shuts down gracefully
status: Done
assignee:
  - TASK-0090
created_date: '2026-06-12 08:31'
updated_date: '2026-06-12 17:44'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
priority: medium
ordinal: 72000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/lib.rs:226-229` (`shutdown_signal`), `crates/server/src/lib.rs:219-222` (`axum::serve(...).with_graceful_shutdown`)

**What**: `shutdown_signal` awaits only `tokio::signal::ctrl_c()` (SIGINT). The crate ships a Debian package with a systemd unit (`[package.metadata.deb.systemd-units]` in `crates/server/Cargo.toml`), and systemd's default `KillSignal` is SIGTERM — for which the process has no handler, so the default disposition (immediate termination) applies. In the packaged deployment, `systemctl stop`/`restart` (including every package upgrade) kills the process abruptly: the graceful-shutdown path wired into `axum::serve` never runs, in-flight requests are dropped mid-response, and the "shutting down" log line never appears. The same applies to `docker stop` (SIGTERM) for the Docker image.

**Why it matters**: The graceful-shutdown machinery exists but is dead code on every supported production path (systemd, Docker); it only works for an interactive Ctrl-C. The fix is the standard `tokio::select!` over `ctrl_c()` and `tokio::signal::unix::signal(SignalKind::terminate())` (unix-gated, with a `ctrl_c`-only fallback elsewhere).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 shutdown_signal resolves on SIGTERM as well as Ctrl-C on unix (tokio::select! over both)
- [x] #2 Non-unix builds still compile (cfg-gated SIGTERM handling)
<!-- AC:END -->
