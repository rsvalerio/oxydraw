---
id: TASK-0112
title: >-
  READ-6: require_folder_access and require_scene_access have divergent
  signatures
status: Done
assignee:
  - TASK-0120
created_date: '2026-06-14 19:06'
updated_date: '2026-06-14 21:53'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
ordinal: 112000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/server/src/ext_routes.rs:78` (`require_folder_access` -> `Result<Folder, Response>`) vs `backend/crates/server/src/ext_routes/scenes.rs:262` (`require_scene_access` -> `Result<(), Box<Response>>`)

**What**: Two sibling access-check helpers for the same purpose have divergent signatures (`Response` vs `Box<Response>`) and divergent success shapes (`Folder` vs `()`), forcing every scene handler to deref (`return *response;`) while folder handlers do not.

**Why it matters**: Cognitive load and copy-paste hazard on the security-critical access-check seam; the Box/non-Box split is the kind of inconsistency that leads a future handler to mis-handle one path and drop the error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The two helpers share a single return convention (both boxed or both not), documented once
- [ ] #2 No handler needs an ad-hoc *response deref that its sibling avoids
<!-- AC:END -->
