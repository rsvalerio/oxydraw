---
id: TASK-0066
title: >-
  DUP-3: i64-to-u64 scalar conversion + Backend map_err repeated across
  SqliteStore count/sum queries
status: Done
assignee:
  - TASK-0088
created_date: '2026-06-11 22:04'
updated_date: '2026-06-12 15:15'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
ordinal: 66000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/storage/src/sqlite.rs:188-193` (`total_payload_bytes`), `:208-215` (`count_scenes`), `:298-304` (`count_files`)

**What**: The shape "run a `query_scalar` returning `i64`, then `u64::try_from(x).map_err(|e| StoreError::Backend(Box::new(e)))` with the same `// ... is non-negative; a conversion failure would mean backend corruption` comment" appears three times. `total_payload_bytes` already extracted the helper for the two `SUM(LENGTH(...))` callers, but `count_scenes` and `count_files` re-inline the identical fetch-and-widen sequence instead of reusing it (their only difference is a bind parameter and the SQL text).

**Why it matters**: Maintainability. The non-negative-scalar-to-u64 contract is encoded by copy-paste in three places; a fourth counter/quota query can drift (e.g. use `as u64` and silently wrap on a corrupt negative). Generalizing the existing helper to "fetch one non-negative i64 scalar, widen to u64" (taking optional binds, or two thin wrappers for bound/unbound) collapses all three sites to one conversion point.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The i64-to-u64 widening with Backend error mapping exists in exactly one function, reused by total_payload_bytes, count_scenes, and count_files
- [x] #2 Store contract tests for document/file byte totals and scene counts still pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Generalized the quota helper into fetch_scalar_u64(pool, QueryScalar<i64>) — the single place that fetches a non-negative scalar (COUNT/SUM) and widens it to u64 with the Backend error mapping. total_payload_bytes was subsumed by it; documents_total_bytes, files_total_bytes, count_scenes (bound query), and count_files all call the one helper. Contract tests for byte totals and scene counts pass; clippy clean.
<!-- SECTION:NOTES:END -->
