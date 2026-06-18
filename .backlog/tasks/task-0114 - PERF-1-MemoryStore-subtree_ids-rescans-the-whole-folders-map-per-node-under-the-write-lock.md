---
id: TASK-0114
title: >-
  PERF-1: MemoryStore subtree_ids rescans the whole folders map per node under
  the write lock
status: Done
assignee:
  - TASK-0122
created_date: '2026-06-14 19:06'
updated_date: '2026-06-15 17:17'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
ordinal: 114000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/storage/src/memory.rs:498` (`subtree_ids`), used by `subtree_height` at `:536`

**What**: `subtree_height` calls `subtree_ids`, which for each popped node iterates the entire `folders` map (`for (child_id, folder) in map.iter()`) to find children; `subtree_height` additionally calls `folder_depth` once per subtree node — an O(n^2)-ish walk under the held `folders` mutex during `move_folder`. This is the in-memory dev/test store, so data sets are small and impact is bounded, but it runs under the write lock.

**Why it matters**: Low severity for the dev/test backend, but the nested full-map scan under a held lock is a latent scaling cliff and worth a note for parity with the SQLite recursive-CTE approach.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 subtree_ids builds a parent_id->children index once instead of rescanning the whole map per node, or documents the small-N assumption
- [ ] #2 subtree_height avoids the redundant per-node folder_depth recomputation
<!-- AC:END -->
