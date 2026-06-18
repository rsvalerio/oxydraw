---
id: TASK-0055
title: >-
  DUP-2: Byte-quota check-then-reject duplicated between document and file write
  paths
status: Done
assignee:
  - TASK-0059
created_date: '2026-06-10 21:10'
updated_date: '2026-06-11 15:44'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
ordinal: 55000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
<!-- scan confidence: candidates to inspect -->

**File**: `crates/server/src/firebase.rs:335`

**What**: The quota pattern — read total bytes from the store (error -> log + internal_error), then `total.saturating_add(payload.len() as u64) > state.config.max_*_bytes` -> warn + (StatusCode::INSUFFICIENT_STORAGE, "storage quota exhausted") — appears at crates/server/src/routes.rs:48-58 (create_document) and crates/server/src/firebase.rs:335-345 (persist_durable_file), including the identical 507 message and the identical "check-then-insert is racy" comment. The scene count quota at ext_routes.rs:626-636 is a third, looser sibling (count-based, different status message).

**Why it matters**: Two near-identical ~11-line blocks differing only in the store method and config field; a shared check_quota helper would keep the 507 contract and saturating-add semantics in one place. Only two close instances, so per DUP-9 this is a judgment call rather than a must-fix.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either extract a small shared quota helper used by create_document and persist_durable_file, or record an explicit decision that the two copies stay (e.g. a cross-referencing comment)
- [ ] #2 If extracted, the 507 status, message, and saturating_add overflow behavior are asserted by the existing share.rs and storage.rs quota tests without modification
<!-- AC:END -->
