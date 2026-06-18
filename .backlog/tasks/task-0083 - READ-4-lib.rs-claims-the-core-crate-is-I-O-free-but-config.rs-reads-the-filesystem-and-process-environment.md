---
id: TASK-0083
title: >-
  READ-4: lib.rs claims the core crate is I/O-free, but config.rs reads the
  filesystem and process environment
status: Done
assignee:
  - TASK-0093
created_date: '2026-06-12 10:32'
updated_date: '2026-06-12 20:32'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
ordinal: 83000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/lib.rs:3` (vs `crates/core/src/config.rs:207-210`)

**What**: The crate-level doc opens with "This crate is I/O-free", but `Config::from_env` reads a `.env` file from disk (`dotenvy::dotenv()`) and the process environment (`std::env::var` in `RawEnv::data`). The accurate claim — and the one the rest of the sentence actually makes — is narrower: the crate contains no *storage backend* I/O.

**Why it matters**: READ-4/READ-5 — crate-level docs are the first thing a contributor trusts when deciding where code may go; a false invariant ("I/O-free") either gets believed (someone refactors config I/O out, or assumes the crate is safe in a no-fs context) or gets ignored (doc rot erodes trust in the remaining docs). One-line fix: scope the claim to "contains no backend / storage I/O" or "the model and store modules are I/O-free".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The lib.rs crate doc no longer claims blanket I/O-freedom; it states the actual invariant (no storage-backend I/O; config reads env/.env)
<!-- AC:END -->
