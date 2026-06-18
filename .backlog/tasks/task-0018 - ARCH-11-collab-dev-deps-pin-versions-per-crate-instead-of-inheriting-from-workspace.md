---
id: TASK-0018
title: >-
  ARCH-11: collab dev-deps pin versions per-crate instead of inheriting from
  workspace
status: Done
assignee:
  - TASK-0028
created_date: '2026-06-07 12:00'
updated_date: '2026-06-07 13:14'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/collab/Cargo.toml:25-27`

**What**: `rust_socketio = "0.6"`, `futures-util = "0.3"`, and `base64 = "0.22"` are pinned directly in the crate manifest while every other dependency in this crate (and the workspace convention generally) inherits via `{ workspace = true }` from `[workspace.dependencies]`.

**Why it matters**: The workspace centralizes versions so CVE bumps and upgrades are a single-point change; per-crate pins are where version drift starts once a second crate needs the same dep (e.g. `base64` or `futures-util` are very likely to appear elsewhere). Low severity: today no sibling crate declares a diverging version of these three.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rust_socketio, futures-util, and base64 are declared in [workspace.dependencies] and inherited with { workspace = true } in crates/collab/Cargo.toml
- [x] #2 cargo check --workspace (or the project QA gate) passes after the move
<!-- AC:END -->
