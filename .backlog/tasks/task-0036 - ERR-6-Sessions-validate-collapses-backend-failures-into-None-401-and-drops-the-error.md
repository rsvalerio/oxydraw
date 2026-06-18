---
id: TASK-0036
title: >-
  ERR-6: Sessions::validate collapses backend failures into None (401) and drops
  the error
status: Done
assignee:
  - TASK-0056
created_date: '2026-06-10 21:08'
updated_date: '2026-06-11 15:06'
labels:
  - code-review-rust
  - idioms
dependencies: []
priority: low
ordinal: 36000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/session.rs:48`

**What**: `validate` returns `Option<String>` and uses `.ok()?` on `find_session` (session.rs:49), so a storage outage is indistinguishable from an unknown/expired token: `require_session` (crates/server/src/ext_routes.rs:123-138) answers 401 instead of 500 and nothing is logged.

**Why it matters**: During a database failure every authenticated user appears logged out (clients may discard their sessions/cookies), and the root cause leaves no trace in the logs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 validate returns Result<Option<String>, StoreError> (or logs the non-NotFound error before mapping to None)
- [ ] #2 require_session maps backend errors to 500, keeping 401 only for genuinely unknown/expired tokens
<!-- AC:END -->
