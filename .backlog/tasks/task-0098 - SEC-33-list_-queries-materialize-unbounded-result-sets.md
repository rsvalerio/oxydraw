---
id: TASK-0098
title: 'SEC-33: list_* queries materialize unbounded result sets'
status: Done
assignee:
  - TASK-0123
created_date: '2026-06-14 15:49'
updated_date: '2026-06-15 19:53'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
ordinal: 98000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/storage/src/sqlite.rs:365`, `:797`; `backend/crates/storage/src/memory.rs:155`

**What**: `list_scenes`, `list_scenes_in_folder`, `list_permissions`, and `list_folders` `fetch_all` into a `Vec` with no `LIMIT`/pagination. An owner/folder with many rows materializes the entire set in memory per request. (Recursive-walk queries are already bounded by `FOLDER_WALK_LIMIT` — good.)

**Why it matters**: Unbounded per-request allocation tied to stored data volume is a latent DoS/memory-pressure vector as data grows.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add pagination (LIMIT/OFFSET or keyset) to the list queries and the store trait, OR document an enforced upper bound on scenes-per-owner / grants-per-folder
- [ ] #2 Apply consistently across both SqliteStore and MemoryStore backends
<!-- AC:END -->
