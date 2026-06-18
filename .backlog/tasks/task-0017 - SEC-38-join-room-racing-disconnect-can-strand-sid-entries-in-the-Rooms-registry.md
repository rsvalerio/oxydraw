---
id: TASK-0017
title: >-
  SEC-38: join-room racing disconnect can strand sid entries in the Rooms
  registry
status: Done
assignee:
  - TASK-0025
created_date: '2026-06-07 12:00'
updated_date: '2026-06-07 12:50'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
<!-- scan confidence: candidates to inspect -->

**File**: `crates/collab/src/lib.rs:160-201` (`on_join_room`) vs `crates/collab/src/lib.rs:223-237` (`on_disconnect`)

**What**: Registry cleanup relies on `on_disconnect`'s `leave_all(sid)` running after every `try_join(room, sid)` for that sid. socketioxide dispatches event handlers as spawned tasks, so a client that sends `join-room` and immediately drops the connection can plausibly have `on_disconnect` → `leave_all` execute *before* the in-flight `on_join_room` reaches `try_join`. In that interleaving, `try_join` inserts a `members` entry and `rooms_by_sid` entry for an already-dead sid, and no later event ever removes them — the room is never emptied, so the "rooms emptied by removal are dropped" invariant (lib.rs:91-93) silently fails. The stale sid also appears forever in `room-user-change` arrays for that room.

**Why it matters**: An unauthenticated client that deliberately loses the race repeatedly grows the registry without bound (each leaked entry pins up to a 256-byte room id ~3×), turning the carefully-capped join path into a slow memory leak, plus permanent ghost members in live rooms. Requires confirming socketioxide's ordering guarantee between event handlers and the disconnect callback — if the library guarantees disconnect runs after all in-flight handlers for the socket complete, downgrade/close this task with a comment documenting the guarantee.

**Suggested fix shape**: have `try_join` (or `on_join_room` after `try_join`) re-check liveness, e.g. consult `socket.connected()` after registering and roll back if false, or perform registration and the connected-check under the same external ordering (the single `Rooms` lock already exists to anchor this).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The ordering guarantee between socketioxide event handlers and on_disconnect is established and documented in the code; if no guarantee exists, registration is made race-safe (e.g. post-insert connected() check with rollback)
- [x] #2 A test (or documented reasoning if untestable) covers the join-then-immediate-disconnect interleaving leaving no residual registry entry
<!-- AC:END -->
