---
id: TASK-0021
title: >-
  TEST-6: on_disconnect's room-user-change emission to remaining members has no
  end-to-end coverage
status: Done
assignee:
  - TASK-0025
created_date: '2026-06-07 12:04'
updated_date: '2026-06-07 12:53'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/collab/src/lib.rs:223-237` (`on_disconnect`); test files `crates/collab/tests/broadcast.rs`, `crates/collab/tests/relay.rs`

**What**: The `Rooms::leave_all` bookkeeping is well unit-tested (lib.rs:244-377), but no integration test exercises the disconnect *handler*: that a socket dropping its connection causes the remaining room members to receive an updated `room-user-change` array without the departed sid. `broadcast.rs` disconnects clients A and B at the end of `join_room_notifies_existing_members` but asserts nothing afterwards; `relay.rs` never disconnects mid-room. The handler wiring (`on_disconnect` registration, `socket.to(room)` emit after the socket left) is exactly the part the unit tests cannot see — and it is the presence feature Excalidraw clients rely on to drop departed collaborators.

**Why it matters**: A regression in handler registration (e.g. a socketioxide upgrade changing `on_disconnect` semantics, the very risk handshake.rs documents) or in the emit-after-leave path would ship silently: every existing test still passes while live rooms accumulate ghost collaborators. This is also AC-adjacent to TASK-0017 (the join/disconnect race), which calls for interleaving coverage on the same path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 An integration test joins two clients to a room, disconnects one, and asserts the remaining client receives room-user-change no longer containing the departed sid
<!-- AC:END -->
