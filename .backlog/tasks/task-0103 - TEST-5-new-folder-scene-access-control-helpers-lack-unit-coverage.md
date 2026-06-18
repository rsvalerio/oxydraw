---
id: TASK-0103
title: 'TEST-5: new folder/scene access-control helpers lack unit coverage'
status: Done
assignee:
  - TASK-0120
created_date: '2026-06-14 15:49'
updated_date: '2026-06-14 21:53'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: low
ordinal: 103000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/server/src/ext_routes/folders.rs`, `backend/crates/server/src/ext_routes/scenes.rs`

**What**: `require_folder_access`, `require_scene_access`, `breadcrumb_to`, and `folder_move_error` carry the tenant-isolation logic but have no `#[cfg(test)]` tests in-module. Integration coverage exists in `tests/folders.rs`, but the cross-org "404-not-403" non-disclosure invariant is the security crux and should be regression-locked at the unit level.

**Why it matters**: The "missing-or-cross-org -> 404, never 403" non-disclosure guarantee must be regression-locked close to the logic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add tests asserting require_folder_access/require_scene_access return 404 for both a missing id and an other-org id, and 500 (not 401/403) on a backend error
- [ ] #2 Add a test for folder_move_error mapping each FolderMoveError variant to its status
<!-- AC:END -->
