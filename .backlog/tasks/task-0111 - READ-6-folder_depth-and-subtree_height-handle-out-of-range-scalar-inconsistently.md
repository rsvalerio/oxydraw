---
id: TASK-0111
title: >-
  READ-6: folder_depth and subtree_height handle out-of-range scalar
  inconsistently
status: Done
assignee:
  - TASK-0122
created_date: '2026-06-14 19:06'
updated_date: '2026-06-15 17:17'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
ordinal: 111000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/storage/src/sqlite.rs:1138` (`folder_depth`) vs `:1157` (`subtree_height`)

**What**: `folder_depth` converts the scalar with `usize::try_from(depth).ok()` — silently turning a negative/sentinel into `None`, which the caller treats as `NotFound`. `subtree_height` converts the same kind of scalar with `usize::try_from(height).map_err(...)` — surfacing an error. The two recursive helpers feed the same `move_folder` depth-budget check but handle the boundary differently, and `folder_depth`'s `.ok()` collapses a genuine conversion failure into the "folder absent" path (a fail-open-ish ambiguity in the tree-integrity check).

**Why it matters**: Inconsistent boundary handling across the two halves of one invariant check is a correctness/readability hazard; `.ok()` masking a real failure as `NotFound` rather than a backend error hides data corruption.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 folder_depth distinguishes 'no such folder' (the COALESCE(...,-1) sentinel) from a genuine try_from conversion failure, consistent with subtree_height
- [ ] #2 Both helpers handle the out-of-range scalar the same way (both error, or both documented as sentinel->None)
<!-- AC:END -->
