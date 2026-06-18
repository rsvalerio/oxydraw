---
id: TASK-0102
title: 'DUP-1: lock_unpoisoned duplicated across collab and core crates'
status: Done
assignee: []
created_date: '2026-06-14 15:49'
updated_date: '2026-06-14 19:22'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
ordinal: 102000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/collab/src/lib.rs:33-37` (vs `backend/crates/core/src/sync.rs:12-14`)

**What**: `collab` re-implements `lock_unpoisoned` byte-for-byte rather than depending on `oxydraw_core::sync::lock_unpoisoned`. An inline comment justifies it ("intentionally stays free of workspace dependencies").

**Why it matters**: Duplicated synchronization primitives can drift in poisoning semantics between the relay and the rest of the backend.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Remove the duplication (shared dependency) OR accept the inline DUP-9 justification and leave as-is with a note
- [x] #2 If kept duplicated, a test or comment pins the two implementations to identical behavior
<!-- AC:END -->
