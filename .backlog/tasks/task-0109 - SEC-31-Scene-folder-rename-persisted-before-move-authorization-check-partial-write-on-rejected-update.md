---
id: TASK-0109
title: >-
  SEC-31: Scene/folder rename persisted before move-authorization check (partial
  write on rejected update)
status: Done
assignee:
  - TASK-0118
created_date: '2026-06-14 19:06'
updated_date: '2026-06-14 20:23'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
ordinal: 109000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/server/src/ext_routes/scenes.rs:225-247` (`update_scene`); same ordering risk in `folders.rs:218-238` (`update_folder`)

**What**: In `update_scene`, a PATCH carrying both `name` and a `folder_id` the caller is NOT authorized to move into applies the rename first, then `require_folder_access` rejects the move with the destination's 404. The client gets an error but the rename has already persisted — a non-atomic, partially-applied update. A caller probing folder existence via the move arm also gets a free rename side effect.

**Why it matters**: Violates fail-closed expectations — a request that returns an error should not have mutated state. Not a cross-tenant breach (rename is gated by scene access), but a correctness/least-surprise defect on a security-relevant seam.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All authorization checks for a PATCH (entity access + destination access) are performed before any mutation; on any failure no field is written
- [ ] #2 A test PATCHes {name, folder_id: <forbidden>} and asserts the name is unchanged after the rejected response
<!-- AC:END -->
