---
id: TASK-0011
title: 'DUP-3: ''.map_err(backend_err)'' repeated 57 times in sqlite.rs'
status: Done
assignee:
  - TASK-0027
created_date: '2026-06-07 11:30'
updated_date: '2026-06-07 13:08'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/storage/src/sqlite.rs` — 57 occurrences (lines 37, 44, 56, 70, 80, 92, 104, 114, 124, 135, 142-148, 159, 162, 175, 193, 208, 226, 238, 250, 271, 280, 283-284, 301-305, 323, 330, 353, 364, 375, 384, 388, 400-401, 411, 428, 439, 442-444, 455, 464, 478, 489, 502, 505-507)

**What**: Every sqlx call and every `try_get` is followed by `.map_err(backend_err)`. The helper is already extracted, but the repetition (57×) is the DUP-3 error-mapping pattern. A `From<sqlx::Error> for StoreError` impl would let plain `?` do the conversion and delete every call. The orphan rule blocks adding the impl in this crate (both types are foreign here), so it must live in `oxydraw-core` behind a feature (e.g. `sqlx-error`), or `StoreError` gains a generic `Backend` constructor pattern that `?` can reach.

**Why it matters**: 57 identical suffixes obscure the actual query logic and each new query is a chance to forget the mapping (compile error, but friction). One `From` impl removes a third of the visual noise in the file.

**Note (DUP-9)**: If the feature-gated impl in core is judged worse than the duplication (core taking an optional sqlx dep), explicitly close as won't-fix with that rationale — the current code is otherwise idiomatic sqlx.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Either a From<sqlx::Error> for StoreError conversion exists (feature-gated in core) and sqlite.rs uses plain '?', or the task is closed won't-fix with the orphan-rule/dependency rationale recorded
- [x] #2 No behavior change; store contract tests pass
<!-- AC:END -->
