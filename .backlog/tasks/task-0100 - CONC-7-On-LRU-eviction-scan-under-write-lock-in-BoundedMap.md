---
id: TASK-0100
title: 'CONC-7: O(n) LRU eviction scan under write lock in BoundedMap'
status: Done
assignee:
  - TASK-0123
created_date: '2026-06-14 15:49'
updated_date: '2026-06-15 19:54'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: low
ordinal: 100000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/server/src/bounded_map.rs:77-84`

**What**: `evict_lru` scans all entries with `min_by_key` on every insert that breaches a bound. Under sustained insert pressure on a near-full map (up to `SCENE_MAX_ENTRIES = 1024`), each insert pays O(n) while holding the exclusive write lock.

**Why it matters**: A linear scan per eviction under the write lock can become a contention/latency hotspot on the (unauthenticated) snapshot-write path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Eviction recency ordering uses a structure giving better-than-O(n) eviction (e.g. intrusive LRU list / BTreeMap keyed by stamp), OR the O(n) cost is explicitly justified against the configured bound
- [ ] #2 A benchmark or comment documents the worst-case eviction cost at SCENE_MAX_ENTRIES
<!-- AC:END -->
