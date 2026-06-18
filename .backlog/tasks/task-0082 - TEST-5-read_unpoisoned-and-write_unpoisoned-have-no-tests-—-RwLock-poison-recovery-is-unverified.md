---
id: TASK-0082
title: >-
  TEST-5: read_unpoisoned and write_unpoisoned have no tests — RwLock poison
  recovery is unverified
status: Done
assignee: []
created_date: '2026-06-12 10:32'
updated_date: '2026-06-12 15:02'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: low
ordinal: 82000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/sync.rs:19-27`

**What**: The `#[cfg(test)]` module covers `lock_unpoisoned` for both the happy path and the poisoned-Mutex recovery path (`sync.rs:33-51`), but the two public RwLock siblings — `read_unpoisoned` and `write_unpoisoned` — have zero tests. Their whole reason to exist is the poisoning edge case, which is exactly what's untested.

**Why it matters**: TEST-5 (every public API function needs at least one test). The poison-recovery contract is the function's documented behavior ("degrade to a usable guard rather than a permanent outage"); a regression (e.g. someone "simplifying" to `.read().unwrap()`) would only surface as a production outage after a panic elsewhere. The existing Mutex test is a ready template: poison via catch_unwind while holding a *write* guard, then assert both helpers still return usable guards.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 read_unpoisoned has a test recovering a usable guard from a poisoned RwLock
- [x] #2 write_unpoisoned has a test recovering a usable (writable) guard from a poisoned RwLock
<!-- AC:END -->
