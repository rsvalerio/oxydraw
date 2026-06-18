---
id: TASK-0019
title: >-
  CL-3: room-user-change member list read under a second lock acquisition, not
  atomic with the join
status: Done
assignee:
  - TASK-0025
created_date: '2026-06-07 12:04'
updated_date: '2026-06-07 12:51'
labels:
  - code-review-rust
  - cognitive-load
dependencies: []
priority: low
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/collab/src/lib.rs:164-200` (`on_join_room`), `crates/collab/src/lib.rs:67-89` (`Rooms::try_join` / `Rooms::members`)

**What**: `on_join_room` calls `rooms.try_join(&room, &sid)` (one lock acquisition, returns only the count) and later `rooms.members(&room)` (a second, independent lock acquisition) to build the `room-user-change` payload. Between the two acquisitions a concurrent join or disconnect can mutate the room, so the emitted member list is not the snapshot that corresponds to this join — the handler implicitly assumes nothing interleaves. `try_join` already holds the lock with the full `IndexSet` in hand; it could return the member snapshot (or `(count, Vec<String>)`) and make the read atomic while also saving a lock round-trip.

**Why it matters**: Under concurrent joins/leaves to the same room, clients can receive `room-user-change` arrays that never reflect a consistent membership state for the triggering event (e.g. two simultaneous joins each emit a list missing the other, and delivery order decides which stale list a client keeps until the next change). Note the fix is a strict improvement but not a total ordering guarantee — emission still happens after the lock is released, and socketioxide delivery order across handlers is not synchronized; document that residual gap at the call site.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 try_join returns the post-join member snapshot taken under the same lock acquisition that performed the insert; on_join_room no longer calls members() separately
- [x] #2 A comment at the emit site documents the remaining emission-ordering caveat (lock released before await)
<!-- AC:END -->
