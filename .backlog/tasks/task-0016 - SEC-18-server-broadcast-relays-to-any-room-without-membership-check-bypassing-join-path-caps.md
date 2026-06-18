---
id: TASK-0016
title: >-
  SEC-18: server-broadcast relays to any room without membership check,
  bypassing join-path caps
status: Done
assignee:
  - TASK-0025
created_date: '2026-06-07 12:00'
updated_date: '2026-06-07 12:46'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/collab/src/lib.rs:208`

**What**: `on_server_broadcast` forwards `client-broadcast` to `socket.to(room)` without verifying the sender ever joined `room` (the `Rooms` registry is not consulted — note the handler doesn't even take `State(rooms)`). The `join-room` path carefully enforces `MAX_ROOM_ID_BYTES` and `MAX_ROOMS_PER_SOCKET` before touching adapter state, but the broadcast path accepts any room id of any length. Any connected (unauthenticated) socket can inject up to 5 MB `client-broadcast` frames into any room id it can guess or enumerate, without that traffic counting against any cap.

**Why it matters**: The relay's only access control is knowledge of the room id (payloads are E2E-encrypted, matching upstream excalidraw-room), so an attacker can't forge meaningful scene data — but they can spam arbitrary rooms with garbage frames (client-side decryption errors / bandwidth DoS on room members) while bypassing the abuse caps the crate deliberately built for the join path. Upstream excalidraw-room has the same gap; this implementation already has the `Rooms` registry to do better at one lock lookup.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 on_server_broadcast drops (or rejects with a warn! log) broadcasts to rooms the sender's sid has not joined, using the Rooms registry
- [x] #2 Room ids on the broadcast path are subject to the same MAX_ROOM_ID_BYTES bound as join-room (cheap early reject before adapter lookup)
- [x] #3 An integration test proves a socket that never joined a room cannot deliver client-broadcast to that room's members
<!-- AC:END -->
