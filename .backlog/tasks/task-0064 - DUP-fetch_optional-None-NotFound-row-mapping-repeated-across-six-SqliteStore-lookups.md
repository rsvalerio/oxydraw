---
id: TASK-0064
title: >-
  DUP: fetch_optional + None=>NotFound row-mapping repeated across six
  SqliteStore lookups
status: Done
assignee:
  - TASK-0088
created_date: '2026-06-11 21:28'
updated_date: '2026-06-12 15:13'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
ordinal: 64000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/storage/src/sqlite.rs` — `find_id` (156-167), `find_scene` (234-246), `get_file` (276-288), `find_session` (432-447), `find_user` (407-417), `org_for_user` (493-510)

**What**: Six single-row lookups repeat the same shape: `sqlx::query(...).bind(...).fetch_optional(&self.pool).await?` followed by `match row { Some(r) => Ok(map(r)), None => Err(StoreError::NotFound) }`. The `None => Err(StoreError::NotFound)` arm is duplicated verbatim at all six sites, with only the Some-arm mapping differing (three already delegate to `scene_from_row`/`user_from_row`).

**Why it matters**: Maintainability, with a correctness edge. TASK-0045 (TEST-11) established that NotFound-vs-other is load-bearing — the HTTP layer maps `NotFound` to 404 and every other error to 500. The empty-row -> NotFound mapping is currently a convention copy-pasted six times; a future lookup that forgets it (or returns a different error) would silently turn a missing row into a 500 with no test catching the shape. Centralizing it makes the contract enforced by construction rather than by copy-paste discipline.

**Fix**: extract a small helper, e.g. `fn require_row<T>(row: Option<SqliteRow>, map: impl FnOnce(&SqliteRow) -> Result<T, StoreError>) -> Result<T, StoreError>` returning `Err(StoreError::NotFound)` on `None`, and route the six lookups through it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The Option-row -> StoreError::NotFound mapping exists in exactly one place, reused by all single-row SqliteStore lookups
- [x] #2 Existing store-contract NotFound assertions still pass for find_id/find_scene/get_file/find_session/find_user/org_for_user
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Extracted require_row(Option<SqliteRow>, map) which is now the single place mapping an absent row to StoreError::NotFound; find_id, find_scene, get_file, find_user, find_session, and org_for_user all route through it. delete_scene's rows_affected()==0 check is intentionally untouched (not a fetch_optional lookup). Store contract tests (incl. assert_not_found! coverage for all six lookups) pass.
<!-- SECTION:NOTES:END -->
