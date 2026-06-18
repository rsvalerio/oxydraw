---
id: TASK-0045
title: >-
  TEST-11: Store contract tests assert is_err() instead of pinning
  StoreError::NotFound
status: Done
assignee:
  - TASK-0060
created_date: '2026-06-10 21:09'
updated_date: '2026-06-11 15:53'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: medium
ordinal: 45000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/storage/src/lib.rs:116`

**What**: The shared store contract asserts missing rows with bare `.is_err()` (lines 116, 159, 171, 193, 259, 286, 289, 309) instead of matching the specific `StoreError::NotFound` variant. Meanwhile the doc comments say "missing ids are NotFound" and the HTTP layer maps NotFound to 404 but Backend errors to 500 (proven load-bearing by crates/server/tests/store_errors.rs).

**Why it matters**: A backend that returns StoreError::Backend for a missing row would pass the entire contract suite yet make the server answer 500 instead of 404 for unknown share links, so the variant distinction is exactly what the contract should pin.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each not-found assertion in test_support (documents, scenes, files, users, sessions, orgs helpers) uses assert!(matches!(err, StoreError::NotFound)) or equivalent instead of bare .is_err()
- [ ] #2 Both MemoryStore and SqliteStore still pass the tightened contract
<!-- AC:END -->
