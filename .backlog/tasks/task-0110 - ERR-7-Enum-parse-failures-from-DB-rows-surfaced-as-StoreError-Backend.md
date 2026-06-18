---
id: TASK-0110
title: 'ERR-7: Enum parse failures from DB rows surfaced as StoreError::Backend'
status: Done
assignee:
  - TASK-0124
created_date: '2026-06-14 19:06'
updated_date: '2026-06-16 15:34'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
ordinal: 110000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/storage/src/sqlite.rs:1415-1417` (`effective_permission`), `:1464`, `:1466-1468` (`list_permissions`)

**What**: When a stored `permission`/`principal_kind` string fails to parse, the code wraps the parse error in `StoreError::Backend(Box::new(e))`. A malformed enum string in the DB is a data-integrity/decoding fault, not a backend/IO fault; bucketing it under `Backend` conflates a corrupt-row condition with transport/connection errors, so a single bad row is indistinguishable from a DB outage to callers.

**Why it matters**: Mapping a domain decode failure onto the generic `Backend` variant degrades diagnosability and error-type precision (map internal errors to the right public type at the boundary).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Enum-parse failures from a DB row map to a dedicated decode/data-integrity error variant, or a documented rationale for reusing Backend
- [x] #2 The mapping is consistent across effective_permission and list_permissions
<!-- AC:END -->
