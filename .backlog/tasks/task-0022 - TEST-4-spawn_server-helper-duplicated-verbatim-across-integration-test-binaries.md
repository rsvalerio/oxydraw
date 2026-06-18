---
id: TASK-0022
title: >-
  TEST-4: spawn_server helper duplicated verbatim across integration test
  binaries
status: Done
assignee:
  - TASK-0025
created_date: '2026-06-07 12:04'
updated_date: '2026-06-07 12:54'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/collab/tests/broadcast.rs:16-25`, `crates/collab/tests/relay.rs:19-28`, inline variant in `crates/collab/tests/handshake.rs:11-18`

**What**: The `spawn_server()` helper (build layer → mount on axum Router → bind 127.0.0.1:0 → spawn `axum::serve` → return base URL) is copy-pasted identically in `broadcast.rs` and `relay.rs`, with a third inline near-copy in `handshake.rs`. Integration tests are separate binaries, so sharing requires the standard `tests/common/mod.rs` (or `tests/common.rs` + `#[path]`) pattern. Per TEST-4, identical setup across 3+ tests should be a shared helper; this is setup boilerplate, not scenario logic, so DUP-10's test-clarity tolerance does not apply.

**Why it matters**: When the server bootstrap changes (e.g. TASK-0016's fix lands and tests need the `Rooms` handle, or an axum/socketioxide upgrade changes the mounting call), three sites must change in lockstep — a drift in one silently tests a different server configuration than the others.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 spawn_server lives in a single shared test module (tests/common/mod.rs or equivalent) consumed by broadcast.rs, relay.rs, and handshake.rs
- [x] #2 cargo test for the collab crate still passes
<!-- AC:END -->
