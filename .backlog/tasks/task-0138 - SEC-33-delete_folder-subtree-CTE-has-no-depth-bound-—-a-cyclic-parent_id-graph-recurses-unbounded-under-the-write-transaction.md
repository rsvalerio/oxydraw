---
id: TASK-0138
title: >-
  SEC-33: delete_folder subtree CTE has no depth bound — a cyclic parent_id
  graph recurses unbounded under the write transaction
status: Done
assignee:
  - TASK-0139
created_date: '2026-06-16 19:06'
updated_date: '2026-06-16 19:17'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
ordinal: 138000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/storage/src/sqlite.rs:778-787` (the `WITH RECURSIVE sub(id) AS (...)` scene-deletion CTE in `delete_folder`)

**What**: The recursive subtree CTE in `delete_folder` is the only `UNION ALL` recursive folder walk in the file with **no depth bound**. The four sibling recursive CTEs all cap the walk at `FOLDER_WALK_LIMIT` via `WHERE depth < ?`: `folder_depth` (~L519), `subtree_height` (~L550), `is_ancestor_or_self` (~L577), and `effective_permission` (~L849). `delete_folder`'s CTE has neither a depth column nor a bound, and SQL `UNION ALL` does no cycle dedup.

**Why it matters**: The codebase's own `FOLDER_WALK_LIMIT` rationale (around L496-500) states the bound exists to stop a corrupt/cyclic `parent_id` graph from looping forever. `folders.parent_id` has no SQL-level cycle constraint — cycles are prevented only by application-layer move/create checks. A corrupt or externally-introduced `parent_id` cycle within the deleted subtree would make this CTE recurse without termination during the `BEGIN IMMEDIATE` transaction, stalling/aborting the delete while holding the write lock — exactly the failure mode every other walk in the module fails closed against. Defense-in-depth gap; low severity because cycles are currently prevented at the application layer.

<!-- scan confidence: verified at source line -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The delete_folder subtree CTE carries the same depth bound (WHERE depth < FOLDER_WALK_LIMIT, with a depth column) as the other four recursive CTEs in the file, so a cyclic parent_id graph terminates the walk instead of looping
- [x] #2 A test seeds a folder row with a self- or mutual-cycle parent_id (inserted directly, bypassing the move-cycle guard) and asserts delete_folder terminates rather than hanging
<!-- AC:END -->
