---
id: TASK-0108
title: 'SEC-20: Store-layer move_folder/move_scene accept a cross-org destination'
status: Done
assignee:
  - TASK-0118
created_date: '2026-06-14 18:57'
updated_date: '2026-06-14 20:23'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
ordinal: 108000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/core/src/store.rs:206` (`move_scene`), `:243` (`move_folder`); isolation enforced only at `backend/crates/server/src/ext_routes/scenes.rs:198` and `folders.rs:240`

**What**: Neither `move_scene` nor `move_folder` validates that the moved entity and the destination/`new_parent` folder belong to the same org. The store will reparent a folder under a parent in a different org. Tenant isolation is enforced solely by the two HTTP handlers calling `require_folder_access` on both ends. `FolderId<'_>` carries no org, so the type system does not prevent a cross-tenant move; any future caller (CLI, batch job, new endpoint) that omits the paired check silently breaks isolation with no defense in depth.

**Why it matters**: The store is the last line of the security boundary. The destination check in the update handler is a separate `if let` block from the move call, so it is easy to omit, turning a single missed check into a cross-tenant data-relocation / IDOR.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 move_folder/move_scene take the caller's OrgId and reject a destination (or entity) outside it with NotFound, OR a store-layer contract test asserts cross-org reparent is rejected
- [ ] #2 A regression test moves a scene/folder into another org's folder via the store API and asserts it fails
<!-- AC:END -->
