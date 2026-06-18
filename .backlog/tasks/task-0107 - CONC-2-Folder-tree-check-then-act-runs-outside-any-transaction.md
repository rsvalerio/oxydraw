---
id: TASK-0107
title: 'CONC-2: Folder tree check-then-act runs outside any transaction'
status: Done
assignee:
  - TASK-0118
created_date: '2026-06-14 18:57'
updated_date: '2026-06-14 20:23'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: medium
ordinal: 107000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/storage/src/sqlite.rs:1258` (`create_folder`), `:1304` (`move_folder`), `:1367` (`effective_permission`)

**What**: `create_folder` and `move_folder` validate invariants (`folder_depth`, `is_ancestor_or_self`, `subtree_height`) with separate pooled queries, then perform the INSERT/UPDATE on a different pooled connection with no enclosing transaction. A concurrent reparent/create can change the tree between the check and the write, so two individually-acyclic reparents can together form a cycle or push depth past `MAX_FOLDER_DEPTH`. `delete_folder` (`:1337`) already does this correctly with `BEGIN IMMEDIATE`. Separately, `effective_permission` reads three independent snapshots (find_folder, org_members EXISTS, recursive ACL walk) across pooled connections, so a concurrent grant/move can yield a torn authorization decision.

**Why it matters**: The cycle/depth invariants are the safety property of the folder tree — the recursive walks are bounded by `FOLDER_WALK_LIMIT` precisely because a corrupt cycle would otherwise loop forever. A corrupt `parent_id` graph degrades every subsequent tree walk, listing, and permission resolution. The permission path is an authorization decision.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 create_folder and move_folder perform their validation reads and the mutating write inside a single BEGIN IMMEDIATE transaction, matching delete_folder
- [ ] #2 A test exercises a concurrent reparent/create scenario demonstrating the tree cannot be driven into a cycle or past MAX_FOLDER_DEPTH
- [ ] #3 effective_permission reads from a single consistent snapshot, or documents why stale reads are acceptable for this authz path
<!-- AC:END -->
