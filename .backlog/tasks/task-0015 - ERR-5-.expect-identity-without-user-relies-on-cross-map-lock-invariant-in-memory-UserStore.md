---
id: TASK-0015
title: >-
  ERR-5: .expect("identity without user") relies on cross-map lock invariant in
  memory UserStore
status: Done
assignee:
  - TASK-0028
created_date: '2026-06-07 11:55'
updated_date: '2026-06-07 13:13'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/storage/src/memory.rs:135`

**What**: `upsert_user_for_identity` does `users.get_mut(&user_id).expect("identity without user")` in non-test production code of the memory backend. The expect is currently infallible because the same critical section that resolves/creates `user_id` also inserts the user row under the held locks — but the invariant spans two maps and depends on lock scoping.

**Why it matters**: A future refactor that splits the lock scope or reorders the inserts silently turns this into a runtime panic. Low severity: either document the invariant at the callsite or restructure to return `StoreError`.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The invariant (an identity entry always has a matching user row) is documented at the callsite, or the lookup returns StoreError instead of panicking
- [x] #2 memory.rs store-contract tests still pass
<!-- AC:END -->
