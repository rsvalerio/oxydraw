---
id: TASK-0009
title: >-
  PERF-12: No indexes on scenes(owner) and sessions(expires_at) — list/prune
  queries do full table scans
status: Done
assignee:
  - TASK-0027
created_date: '2026-06-07 11:29'
updated_date: '2026-06-07 13:04'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/storage/src/sqlite.rs:50-137` (schema in `init()`); queried at `sqlite.rs:200-210` (`list_scenes`) and `sqlite.rs:459-466` (`prune_sessions`)

**What**: The schema only has the implicit primary-key indexes. Two queries filter on non-indexed columns:
- `list_scenes`: `WHERE owner = ? ORDER BY updated_at DESC` — full scan of `scenes` plus a sort on every library listing.
- `prune_sessions`: `DELETE FROM sessions WHERE expires_at <= ?` — full scan of `sessions` on every prune tick.

**Why it matters**: Both are recurring hot-path queries (every library page load; periodic prune). An index on `scenes(owner, updated_at DESC)` serves the list query fully (filter + order); an index on `sessions(expires_at)` makes pruning proportional to expired rows. Low severity at embedded scale, but the fix is two `CREATE INDEX IF NOT EXISTS` statements in `init()`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 init() creates an index covering scenes(owner, updated_at) used by list_scenes
- [x] #2 init() creates an index on sessions(expires_at) used by prune_sessions
- [x] #3 Indexes use CREATE INDEX IF NOT EXISTS so existing databases pick them up on next start
<!-- AC:END -->
