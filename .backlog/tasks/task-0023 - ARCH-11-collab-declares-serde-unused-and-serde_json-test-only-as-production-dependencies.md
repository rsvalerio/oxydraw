---
id: TASK-0023
title: >-
  ARCH-11: collab declares serde (unused) and serde_json (test-only) as
  production dependencies
status: Done
assignee:
  - TASK-0028
created_date: '2026-06-07 12:05'
updated_date: '2026-06-07 13:14'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/collab/Cargo.toml:14-15`

**What**: `[dependencies]` lists `serde` and `serde_json`, but `src/lib.rs` uses neither: the only `serde_json` mention is in a doc comment (lib.rs:205), and the `Data<String>` / `Data<(String, Bytes, Bytes)>` extractors get their `Deserialize` impls from socketioxide's own serde dependency — this crate writes no derives. `serde_json` *is* used, but only by `tests/relay.rs` and `tests/broadcast.rs`, so it belongs in `[dev-dependencies]`; `serde` appears fully removable. Distinct from TASK-0018, which covers dev-deps pinning versions instead of inheriting from the workspace — this is about deps being in the wrong section (or present at all).

**Why it matters**: Unused production dependencies inflate the audited supply-chain surface (`cargo audit`/`cargo deny` scope), and contradict the crate's own stated goal of a minimal dependency footprint (lib.rs:31-32 keeps it free even of workspace-internal crates). Cheap to verify: `cargo check -p oxydraw-collab` after removal.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 serde is removed from crates/collab/Cargo.toml (or its concrete production use is identified and documented)
- [x] #2 serde_json moves to [dev-dependencies]
- [x] #3 cargo check and cargo test for the collab crate pass after the change
<!-- AC:END -->
