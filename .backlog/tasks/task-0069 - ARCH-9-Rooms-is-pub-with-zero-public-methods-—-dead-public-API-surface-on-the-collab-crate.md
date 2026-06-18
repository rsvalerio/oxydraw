---
id: TASK-0069
title: >-
  ARCH-9: Rooms is pub with zero public methods — dead public API surface on the
  collab crate
status: Done
assignee:
  - TASK-0089
created_date: '2026-06-12 07:05'
updated_date: '2026-06-12 15:25'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
ordinal: 69000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/collab/src/lib.rs:54`

**What**: `pub struct Rooms(Arc<Mutex<RoomsInner>>)` is exported from the crate, but every one of its methods (`try_join`, `leave_all`, `is_member`, plus the `#[cfg(test)]` helpers) is private, and the only function the crate exposes — `build()` — does not mention `Rooms` in its signature. The single external consumer (`crates/server/src/lib.rs:201`) uses only `oxydraw_collab::build()`. Because of `#[derive(Clone, Default)]`, downstream crates can construct and clone a `Rooms` they can do nothing with.

**Why it matters**: Dead public surface is API a maintainer must treat as semver-relevant for no benefit, and it misleads readers into thinking the registry is meant to be driven externally. The type only needs to be visible to socketioxide's `State` extractor, which has no `pub` requirement — all handlers live in this crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Rooms is no longer part of the crate's public API (private or pub(crate))
- [x] #2 crate builds and all collab tests pass; build() remains the sole public item
<!-- AC:END -->
