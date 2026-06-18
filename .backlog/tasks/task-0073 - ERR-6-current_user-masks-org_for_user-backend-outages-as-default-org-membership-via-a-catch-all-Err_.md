---
id: TASK-0073
title: >-
  ERR-6: current_user masks org_for_user backend outages as default-org
  membership via a catch-all Err(_)
status: Done
assignee:
  - TASK-0091
created_date: '2026-06-12 08:32'
updated_date: '2026-06-12 17:56'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
priority: low
ordinal: 73000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/ext_routes/auth.rs:129-134`

**What**: `current_user` resolves the signed-in user's org with `match state.store.org_for_user(...) { Ok(org) => ..., Err(_) => (DEFAULT_ORG, DEFAULT_ORG_NAME) }`. The comment justifies the fallback for a *missing membership* ("a user without one still belongs in the default org rather than locked out") — that case is `StoreError::NotFound`. But the catch-all `Err(_)` also swallows `StoreError::Backend` (storage outage, I/O failure), silently substituting default-org data with no log line. This is the same error-collapse class the codebase already fixed twice: `Sessions::validate` (task-0036, now `Err(StoreError::NotFound) => Ok(None)` with other errors propagated, session.rs:55-62) and `require_session` (auth.rs:94-99, which 500s on backend errors).

**Why it matters**: During a partial storage outage the session lookup may succeed while the org lookup fails; the request then proceeds with fabricated org data and the outage leaves no trace — exactly the silent-degradation failure mode ERR-6 targets. The fix is mechanical: match `Err(StoreError::NotFound)` for the documented fallback and propagate (or at minimum `error!`-log) every other error, mirroring `Sessions::validate`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 org_for_user's NotFound is the only error mapped to the default-org fallback
- [x] #2 Backend errors from org_for_user either propagate as the existing Err arm (surfacing as 500 in require_session) or are explicitly logged as errors
<!-- AC:END -->
