---
id: TASK-0035
title: 'ERR-1: Sessions::revoke silently swallows delete_session errors'
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
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/session.rs:53`

**What**: `revoke` discards the store result with `let _ = self.store.delete_session(...).await;` (session.rs:54) with no log and no comment documenting why the error is safe to ignore. The `logout` handler (crates/server/src/ext_routes.rs:212-222) then returns 204 unconditionally.

**Why it matters**: If the backend write fails, the user believes they are logged out while the session row stays valid until TTL expiry, and the failure is invisible to operators.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A failed delete_session is logged at warn/error level with the store error chain, or revoke returns the Result for the handler to log
- [ ] #2 Intentional discard, if kept, carries a comment justifying it
<!-- AC:END -->
